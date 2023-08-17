import {
  PublicKey,
  Connection,
  Transaction,
  StakeProgram,
  SystemProgram,
  StakeAuthorizationLayout,
} from '@solana/web3.js';
import {
  MINT_SIZE,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  createApproveInstruction,
} from '@solana/spl-token';

import {
  SINGLE_POOL_PROGRAM_ID,
  findPoolAddress,
  findPoolStakeAddress,
  findPoolMintAddress,
  findPoolStakeAuthorityAddress,
  findPoolMintAuthorityAddress,
} from './addresses';
import { SinglePoolInstruction } from './instructions';

export async function initialize(
  connection: Connection,
  voteAccount: PublicKey,
  payer: PublicKey,
  skipMetadata = false,
) {
  const transaction = new Transaction();

  const programId = SINGLE_POOL_PROGRAM_ID;
  const pool = findPoolAddress(programId, voteAccount);
  const stake = findPoolStakeAddress(programId, pool);
  const mint = findPoolMintAddress(programId, pool);

  const poolRent = await connection.getMinimumBalanceForRentExemption(33); // FIXME get buffer size in js
  const stakeRent = await connection.getMinimumBalanceForRentExemption(StakeProgram.space);
  const mintRent = await connection.getMinimumBalanceForRentExemption(MINT_SIZE);
  const minimumDelegation = (await connection.getStakeMinimumDelegation()).value;

  transaction.add(
    SystemProgram.transfer({
      fromPubkey: payer,
      toPubkey: pool,
      lamports: poolRent,
    }),
  );

  transaction.add(
    SystemProgram.transfer({
      fromPubkey: payer,
      toPubkey: stake,
      lamports: stakeRent + minimumDelegation,
    }),
  );

  transaction.add(
    SystemProgram.transfer({
      fromPubkey: payer,
      toPubkey: mint,
      lamports: mintRent,
    }),
  );

  transaction.add(SinglePoolInstruction.initializePool(voteAccount));

  if (!skipMetadata) {
    transaction.add(SinglePoolInstruction.createTokenMetadata(pool, payer));
  }

  return transaction;
}

interface DepositParams {
  connection: Connection;
  pool: PublicKey;
  userWallet: PublicKey;
  userStakeAccount: PublicKey;
  userTokenAccount?: PublicKey;
  userLamportAccount?: PublicKey;
  userWithdrawAuthority?: PublicKey;
}

export async function deposit(params: DepositParams) {
  const { connection, pool, userWallet, userStakeAccount } = params;

  const transaction = new Transaction();

  const programId = SINGLE_POOL_PROGRAM_ID;
  const mint = findPoolMintAddress(programId, pool);
  const poolStakeAuthority = findPoolStakeAuthorityAddress(programId, pool);
  const userAssociatedTokenAccount = getAssociatedTokenAddressSync(mint, userWallet);

  const userTokenAccount = params.userTokenAccount || userAssociatedTokenAccount;
  const userLamportAccount = params.userLamportAccount || userWallet;
  const userWithdrawAuthority = params.userWithdrawAuthority || userWallet;

  if (
    userTokenAccount.equals(userAssociatedTokenAccount) &&
    (await connection.getAccountInfo(userAssociatedTokenAccount)) == null
  ) {
    transaction.add(
      createAssociatedTokenAccountInstruction(
        userWallet,
        userAssociatedTokenAccount,
        userWallet,
        mint,
      ),
    );
  }

  // TODO check token and stake account balances?

  transaction.add(
    StakeProgram.authorize({
      stakePubkey: userStakeAccount,
      authorizedPubkey: userWithdrawAuthority,
      newAuthorizedPubkey: poolStakeAuthority,
      stakeAuthorizationType: StakeAuthorizationLayout.Staker,
    }),
  );

  transaction.add(
    StakeProgram.authorize({
      stakePubkey: userStakeAccount,
      authorizedPubkey: userWithdrawAuthority,
      newAuthorizedPubkey: poolStakeAuthority,
      stakeAuthorizationType: StakeAuthorizationLayout.Withdrawer,
    }),
  );

  transaction.add(
    SinglePoolInstruction.depositStake(
      pool,
      userStakeAccount,
      userTokenAccount,
      userLamportAccount,
    ),
  );

  return transaction;
}

interface WithdrawParams {
  connection: Connection;
  pool: PublicKey;
  userWallet: PublicKey;
  userStakeAccount: PublicKey;
  tokenAmount: number | bigint;
  createStakeAccount?: boolean;
  userStakeAuthority?: PublicKey;
  userTokenAccount?: PublicKey;
  userTokenAuthority?: PublicKey;
}

export async function withdraw(params: WithdrawParams) {
  const { connection, pool, userWallet, userStakeAccount, tokenAmount, createStakeAccount } =
    params;

  const transaction = new Transaction();

  const programId = SINGLE_POOL_PROGRAM_ID;
  const poolMintAuthority = findPoolMintAuthorityAddress(programId, pool);

  const userStakeAuthority = params.userStakeAuthority || userWallet;
  const userTokenAccount =
    params.userTokenAccount ||
    getAssociatedTokenAddressSync(findPoolMintAddress(programId, pool), userWallet);
  const userTokenAuthority = params.userTokenAuthority || userWallet;

  if (createStakeAccount) {
    transaction.add(
      SystemProgram.createAccount({
        fromPubkey: userWallet,
        lamports: await connection.getMinimumBalanceForRentExemption(StakeProgram.space),
        newAccountPubkey: userStakeAccount,
        programId: StakeProgram.programId,
        space: StakeProgram.space,
      }),
    );
  }

  // TODO check token balance?

  transaction.add(
    createApproveInstruction(userTokenAccount, poolMintAuthority, userTokenAuthority, tokenAmount),
  );

  transaction.add(
    SinglePoolInstruction.withdrawStake(
      pool,
      userStakeAccount,
      userStakeAuthority,
      userTokenAccount,
      userTokenAuthority,
      tokenAmount,
    ),
  );

  return transaction;
}

export function createTokenMetadata(pool: PublicKey, payer: PublicKey) {
  const transaction = new Transaction();
  transaction.add(SinglePoolInstruction.createTokenMetadata(pool, payer));

  return transaction;
}

export function updateTokenMetadata(
  voteAccount: PublicKey,
  authorizedWithdrawer: PublicKey,
  name: string,
  symbol: string,
  uri?: string,
) {
  const transaction = new Transaction();
  transaction.add(
    SinglePoolInstruction.updateTokenMetadata(voteAccount, authorizedWithdrawer, name, symbol, uri),
  );

  return transaction;
}
