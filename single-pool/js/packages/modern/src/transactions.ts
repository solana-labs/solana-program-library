import {
  appendTransactionInstruction,
  Transaction,
  TransactionVersion,
  Base58EncodedAddress,
} from '@solana/web3.js';

import {
  findPoolAddress,
  VoteAccountAddress,
  PoolAddress,
  findPoolStakeAddress,
  findPoolMintAddress,
  defaultDepositAccountSeed,
  findDefaultDepositAccountAddress,
  findPoolMintAuthorityAddress,
  findPoolStakeAuthorityAddress,
  SINGLE_POOL_PROGRAM_ID,
} from './addresses.js';
import {
  initializePoolInstruction,
  reactivatePoolStakeInstruction,
  depositStakeInstruction,
  withdrawStakeInstruction,
  createTokenMetadataInstruction,
  updateTokenMetadataInstruction,
} from './instructions.js';
import {
  STAKE_PROGRAM_ID,
  STAKE_ACCOUNT_SIZE,
  MINT_SIZE,
  StakeInstruction,
  SystemInstruction,
  TokenInstruction,
  StakeAuthorizationType,
  getAssociatedTokenAddress,
} from './quarantine.js';

interface DepositParams {
  rpc: any; // XXX Rpc<???>
  pool: PoolAddress;
  userWallet: Base58EncodedAddress;
  userStakeAccount?: Base58EncodedAddress;
  depositFromDefaultAccount?: boolean;
  userTokenAccount?: Base58EncodedAddress;
  userLamportAccount?: Base58EncodedAddress;
  userWithdrawAuthority?: Base58EncodedAddress;
}

interface WithdrawParams {
  rpc: any; // XXX Rpc<???>
  pool: PoolAddress;
  userWallet: Base58EncodedAddress;
  userStakeAccount: Base58EncodedAddress;
  tokenAmount: bigint;
  createStakeAccount?: boolean;
  userStakeAuthority?: Base58EncodedAddress;
  userTokenAccount?: Base58EncodedAddress;
  userTokenAuthority?: Base58EncodedAddress;
}

export const SINGLE_POOL_ACCOUNT_SIZE = 33n;

export const SinglePoolProgram = {
  programAddress: SINGLE_POOL_PROGRAM_ID,
  space: SINGLE_POOL_ACCOUNT_SIZE,
  initialize: initializeTransaction,
  reactivatePoolStake: reactivatePoolStakeTransaction,
  deposit: depositTransaction,
  withdraw: withdrawTransaction,
  createTokenMetadata: createTokenMetadataTransaction,
  updateTokenMetadata: updateTokenMetadataTransaction,
  createAndDelegateUserStake: createAndDelegateUserStakeTransaction,
};

export async function initializeTransaction(
  rpc: any, // XXX not exported: Rpc<???>,
  voteAccount: VoteAccountAddress,
  payer: Base58EncodedAddress,
  skipMetadata = false,
): Promise<Transaction> {
  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };

  const pool = await findPoolAddress(SINGLE_POOL_PROGRAM_ID, voteAccount);
  const [stake, mint, poolRent, stakeRent, mintRent, minimumDelegationObj] = await Promise.all([
    findPoolStakeAddress(SINGLE_POOL_PROGRAM_ID, pool),
    findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, pool),
    rpc.getMinimumBalanceForRentExemption(SINGLE_POOL_ACCOUNT_SIZE).send(),
    rpc.getMinimumBalanceForRentExemption(STAKE_ACCOUNT_SIZE).send(),
    rpc.getMinimumBalanceForRentExemption(MINT_SIZE).send(),
    rpc.getStakeMinimumDelegation().send(),
  ]);
  const minimumDelegation = minimumDelegationObj.value;

  transaction = appendTransactionInstruction(
    SystemInstruction.transfer({
      from: payer,
      to: pool,
      lamports: poolRent,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    SystemInstruction.transfer({
      from: payer,
      to: stake,
      lamports: stakeRent + minimumDelegation,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    SystemInstruction.transfer({
      from: payer,
      to: mint,
      lamports: mintRent,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    await initializePoolInstruction(voteAccount),
    transaction,
  );

  if (!skipMetadata) {
    transaction = appendTransactionInstruction(
      await createTokenMetadataInstruction(pool, payer),
      transaction,
    );
  }

  return transaction;
}

export async function reactivatePoolStakeTransaction(
  voteAccount: VoteAccountAddress,
): Promise<Transaction> {
  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };
  transaction = appendTransactionInstruction(
    await reactivatePoolStakeInstruction(voteAccount),
    transaction,
  );

  return transaction;
}

export async function depositTransaction(params: DepositParams) {
  const { rpc, pool, userWallet } = params;

  // note this is just xnor
  if (!params.userStakeAccount == !params.depositFromDefaultAccount) {
    throw 'must either provide userStakeAccount or true depositFromDefaultAccount';
  }

  const userStakeAccount = (
    params.depositFromDefaultAccount
      ? await findDefaultDepositAccountAddress(pool, userWallet)
      : params.userStakeAccount
  ) as Base58EncodedAddress;

  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };

  const [mint, poolStakeAuthority] = await Promise.all([
    findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, pool),
    findPoolStakeAuthorityAddress(SINGLE_POOL_PROGRAM_ID, pool),
  ]);

  const userAssociatedTokenAccount = await getAssociatedTokenAddress(mint, userWallet);
  const userTokenAccount = params.userTokenAccount || userAssociatedTokenAccount;
  const userLamportAccount = params.userLamportAccount || userWallet;
  const userWithdrawAuthority = params.userWithdrawAuthority || userWallet;

  if (
    userTokenAccount == userAssociatedTokenAccount &&
    (await rpc.getAccountInfo(userAssociatedTokenAccount).send()) == null
  ) {
    transaction = appendTransactionInstruction(
      TokenInstruction.createAssociatedTokenAccount({
        payer: userWallet,
        associatedAccount: userAssociatedTokenAccount,
        owner: userWallet,
        mint,
      }),
      transaction,
    );
  }

  transaction = appendTransactionInstruction(
    StakeInstruction.authorize({
      stakeAccount: userStakeAccount,
      authorized: userWithdrawAuthority,
      newAuthorized: poolStakeAuthority,
      authorizationType: StakeAuthorizationType.Staker,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    StakeInstruction.authorize({
      stakeAccount: userStakeAccount,
      authorized: userWithdrawAuthority,
      newAuthorized: poolStakeAuthority,
      authorizationType: StakeAuthorizationType.Withdrawer,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    await depositStakeInstruction(pool, userStakeAccount, userTokenAccount, userLamportAccount),
    transaction,
  );

  return transaction;
}

export async function withdrawTransaction(params: WithdrawParams) {
  const { rpc, pool, userWallet, userStakeAccount, tokenAmount, createStakeAccount } = params;

  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };

  const poolMintAuthority = await findPoolMintAuthorityAddress(SINGLE_POOL_PROGRAM_ID, pool);

  const userStakeAuthority = params.userStakeAuthority || userWallet;
  const userTokenAccount =
    params.userTokenAccount ||
    (await getAssociatedTokenAddress(
      await findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, pool),
      userWallet,
    ));
  const userTokenAuthority = params.userTokenAuthority || userWallet;

  if (createStakeAccount) {
    transaction = appendTransactionInstruction(
      SystemInstruction.createAccount({
        from: userWallet,
        lamports: await rpc.getMinimumBalanceForRentExemption(STAKE_ACCOUNT_SIZE).send(),
        newAccount: userStakeAccount,
        programAddress: STAKE_PROGRAM_ID,
        space: STAKE_ACCOUNT_SIZE,
      }),
      transaction,
    );
  }

  transaction = appendTransactionInstruction(
    TokenInstruction.approve({
      account: userTokenAccount,
      delegate: poolMintAuthority,
      owner: userTokenAuthority,
      amount: tokenAmount,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    await withdrawStakeInstruction(
      pool,
      userStakeAccount,
      userStakeAuthority,
      userTokenAccount,
      tokenAmount,
    ),
    transaction,
  );

  return transaction;
}

export async function createTokenMetadataTransaction(
  pool: PoolAddress,
  payer: Base58EncodedAddress,
): Promise<Transaction> {
  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };
  transaction = appendTransactionInstruction(
    await createTokenMetadataInstruction(pool, payer),
    transaction,
  );

  return transaction;
}

export async function updateTokenMetadataTransaction(
  voteAccount: VoteAccountAddress,
  authorizedWithdrawer: Base58EncodedAddress,
  name: string,
  symbol: string,
  uri?: string,
): Promise<Transaction> {
  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };
  transaction = appendTransactionInstruction(
    await updateTokenMetadataInstruction(voteAccount, authorizedWithdrawer, name, symbol, uri),
    transaction,
  );

  return transaction;
}

export async function createAndDelegateUserStakeTransaction(
  rpc: any, // XXX not exported: Rpc<???>,
  voteAccount: VoteAccountAddress,
  userWallet: Base58EncodedAddress,
  stakeAmount: bigint,
): Promise<Transaction> {
  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };

  const pool = await findPoolAddress(SINGLE_POOL_PROGRAM_ID, voteAccount);
  const [stakeAccount, stakeRent] = await Promise.all([
    findDefaultDepositAccountAddress(pool, userWallet),
    await rpc.getMinimumBalanceForRentExemption(STAKE_ACCOUNT_SIZE).send(),
  ]);

  transaction = appendTransactionInstruction(
    SystemInstruction.createAccountWithSeed({
      base: userWallet,
      from: userWallet,
      lamports: stakeAmount + stakeRent,
      newAccount: stakeAccount,
      programAddress: STAKE_PROGRAM_ID,
      seed: defaultDepositAccountSeed(pool),
      space: STAKE_ACCOUNT_SIZE,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    StakeInstruction.initialize({
      stakeAccount,
      staker: userWallet,
      withdrawer: userWallet,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    StakeInstruction.delegate({
      stakeAccount,
      authorized: userWallet,
      voteAccount,
    }),
    transaction,
  );

  return transaction;
}
