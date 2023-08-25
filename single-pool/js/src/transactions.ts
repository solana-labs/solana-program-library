import {
  PublicKey,
  Connection,
  Authorized,
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
  findDefaultDepositAccountAddress,
  findPoolAddress,
  findPoolStakeAddress,
  findPoolMintAddress,
  findPoolStakeAuthorityAddress,
  findPoolMintAuthorityAddress,
} from './addresses';
import { SinglePoolInstruction } from './instructions';
import { SINGLE_POOL_PROGRAM_ID, defaultDepositAccountSeed } from './internal';

interface DepositParams {
  connection: Connection;
  pool: PublicKey;
  userWallet: PublicKey;
  userStakeAccount?: PublicKey;
  depositFromDefaultAccount?: boolean;
  userTokenAccount?: PublicKey;
  userLamportAccount?: PublicKey;
  userWithdrawAuthority?: PublicKey;
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

export class SinglePoolProgram {
  static programId: PublicKey = SINGLE_POOL_PROGRAM_ID;

  static space: number = 33;

  static async initialize(
    connection: Connection,
    voteAccount: PublicKey,
    payer: PublicKey,
    skipMetadata = false,
  ) {
    const transaction = new Transaction();

    const pool = findPoolAddress(this.programId, voteAccount);
    const stake = findPoolStakeAddress(this.programId, pool);
    const mint = findPoolMintAddress(this.programId, pool);

    const poolRent = await connection.getMinimumBalanceForRentExemption(this.space);
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

  static async deposit(params: DepositParams) {
    const { connection, pool, userWallet } = params;

    // note this is just "if not xor"
    if (!params.userStakeAccount == !params.depositFromDefaultAccount) {
      throw 'must either provide userStakeAccount or true depositFromDefaultAccount';
    }

    const userStakeAccount = params.depositFromDefaultAccount
      ? await findDefaultDepositAccountAddress(pool, userWallet)
      : params.userStakeAccount;

    const transaction = new Transaction();

    const mint = findPoolMintAddress(this.programId, pool);
    const poolStakeAuthority = findPoolStakeAuthorityAddress(this.programId, pool);
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

    transaction.add(
      StakeProgram.authorize({
        stakePubkey: userStakeAccount as PublicKey,
        authorizedPubkey: userWithdrawAuthority,
        newAuthorizedPubkey: poolStakeAuthority,
        stakeAuthorizationType: StakeAuthorizationLayout.Staker,
      }),
    );

    transaction.add(
      StakeProgram.authorize({
        stakePubkey: userStakeAccount as PublicKey,
        authorizedPubkey: userWithdrawAuthority,
        newAuthorizedPubkey: poolStakeAuthority,
        stakeAuthorizationType: StakeAuthorizationLayout.Withdrawer,
      }),
    );

    transaction.add(
      SinglePoolInstruction.depositStake(
        pool,
        userStakeAccount as PublicKey,
        userTokenAccount,
        userLamportAccount,
      ),
    );

    return transaction;
  }

  static async withdraw(params: WithdrawParams) {
    const { connection, pool, userWallet, userStakeAccount, tokenAmount, createStakeAccount } =
      params;

    const transaction = new Transaction();

    const poolMintAuthority = findPoolMintAuthorityAddress(this.programId, pool);

    const userStakeAuthority = params.userStakeAuthority || userWallet;
    const userTokenAccount =
      params.userTokenAccount ||
      getAssociatedTokenAddressSync(findPoolMintAddress(this.programId, pool), userWallet);
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

    transaction.add(
      createApproveInstruction(
        userTokenAccount,
        poolMintAuthority,
        userTokenAuthority,
        tokenAmount,
      ),
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

  static createTokenMetadata(pool: PublicKey, payer: PublicKey) {
    const transaction = new Transaction();
    transaction.add(SinglePoolInstruction.createTokenMetadata(pool, payer));

    return transaction;
  }

  static updateTokenMetadata(
    voteAccount: PublicKey,
    authorizedWithdrawer: PublicKey,
    name: string,
    symbol: string,
    uri?: string,
  ) {
    const transaction = new Transaction();
    transaction.add(
      SinglePoolInstruction.updateTokenMetadata(
        voteAccount,
        authorizedWithdrawer,
        name,
        symbol,
        uri,
      ),
    );

    return transaction;
  }

  static async createAndDelegateUserStake(
    connection: Connection,
    voteAccount: PublicKey,
    userWallet: PublicKey,
    stakeAmount: number | bigint,
  ) {
    const transaction = new Transaction();

    const pool = findPoolAddress(this.programId, voteAccount);
    const stakeAccount = await findDefaultDepositAccountAddress(pool, userWallet);

    const stakeRent = await connection.getMinimumBalanceForRentExemption(StakeProgram.space);

    // web3.js only supports number, so if amount is a bigint, we check that the conversion will be safe
    if (
      typeof stakeAmount == 'bigint' &&
      stakeAmount + BigInt(stakeRent) > BigInt(Number.MAX_SAFE_INTEGER)
    ) {
      throw 'cannot convert stakeAmount to Number';
    }

    transaction.add(
      SystemProgram.createAccountWithSeed({
        basePubkey: userWallet,
        fromPubkey: userWallet,
        lamports: Number(stakeAmount) + stakeRent,
        newAccountPubkey: stakeAccount,
        programId: StakeProgram.programId,
        seed: defaultDepositAccountSeed(pool),
        space: StakeProgram.space,
      }),
    );

    transaction.add(
      StakeProgram.initialize({
        authorized: new Authorized(userWallet, userWallet),
        stakePubkey: stakeAccount,
      }),
    );

    transaction.add(
      StakeProgram.delegate({
        authorizedPubkey: userWallet,
        stakePubkey: stakeAccount,
        votePubkey: voteAccount,
      }),
    );

    return transaction;
  }
}
