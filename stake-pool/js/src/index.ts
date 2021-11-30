import {
  AccountInfo,
  Connection,
  Keypair,
  PublicKey,
  Signer,
  SystemProgram,
  TransactionInstruction,
} from '@solana/web3.js';
import {
  addAssociatedTokenAccount,
  amountToUiAmount,
  calcLamportsWithdrawAmount,
  findStakeProgramAddress,
  findWithdrawAuthorityProgramAddress,
  getTokenAccount,
  getTokenMint,
  lamportsToSol,
  newStakeAccount,
  prepareWithdrawAccounts,
  solToLamports,
} from './utils';
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, Token } from '@solana/spl-token';
import {
  MIN_STAKE_BALANCE,
  STAKE_STATE_LEN,
  StakePoolProgram,
  WithdrawAccount,
} from './stakepool-program';
import { STAKE_POOL_LAYOUT, VALIDATOR_LIST_LAYOUT, ValidatorList, StakePool } from './layouts';

export type { StakePool, AccountType, ValidatorList, ValidatorStakeInfo } from './layouts';

export interface ValidatorListAccount {
  pubkey: PublicKey;
  account: AccountInfo<ValidatorList>;
}

export interface StakePoolAccount {
  pubkey: PublicKey;
  account: AccountInfo<StakePool>;
}

export interface StakePoolAccounts {
  /**
   * Wrapper class for a stake pool.
   * Each stake pool has a stake pool account and a validator list account.
   */
  stakePool: StakePoolAccount | undefined;
  validatorList: ValidatorListAccount | undefined;
}

export async function getStakePoolInfo(
  connection: Connection,
  stakePoolPubkey: PublicKey,
): Promise<string | null | undefined> {
  const stakePoolAccount = await getStakePoolAccount(connection, stakePoolPubkey);
  const validatorList = await getValidatorListAccount(
    connection,
    stakePoolAccount.account.data.validatorList,
  );
  const mintInfo = await getTokenMint(connection, stakePoolAccount.account.data.poolMint);

  return (
    'Stake Pool Info \n' +
    '=============== \n' +
    'Stake Pool: ' +
    prettyPrintPubKey(stakePoolPubkey) +
    '\n' +
    'Validator List: ' +
    validatorList?.account.data.validators.toString() +
    '\n' +
    'Pool Token Mint: ' +
    mintInfo?.mintAuthority
  );
}

/**
 * Retrieves and deserializes a StakePool account using a web3js connection and the stake pool address.
 * @param connection: An active web3js connection.
 * @param stakePoolPubKey: The public key (address) of the stake pool account.
 */
export async function getStakePoolAccount(
  connection: Connection,
  stakePoolPubKey: PublicKey,
): Promise<StakePoolAccount> {
  const account = await connection.getAccountInfo(stakePoolPubKey);

  if (!account) {
    throw new Error('Invalid account');
  }

  return {
    pubkey: stakePoolPubKey,
    account: {
      data: STAKE_POOL_LAYOUT.decode(account.data),
      executable: account.executable,
      lamports: account.lamports,
      owner: account.owner,
    },
  };
}

/**
 * Retrieves and deserializes a ValidatorList account using a web3js connection and the validator list address.
 * @param connection: An active web3js connection.
 * @param validatorListPubKey: The public key (address) of the validator list account.
 */
export async function getValidatorListAccount(
  connection: Connection,
  validatorListPubKey: PublicKey,
): Promise<ValidatorListAccount | undefined> {
  const account = await connection.getAccountInfo(validatorListPubKey);

  if (!account) {
    throw new Error('Invalid account');
  }

  return {
    pubkey: validatorListPubKey,
    account: {
      data: VALIDATOR_LIST_LAYOUT.decode(account.data),
      executable: account.executable,
      lamports: account.lamports,
      owner: account.owner,
    },
  };
}

/**
 * Retrieves all StakePool and ValidatorList accounts that are running a particular StakePool program.
 * @param connection: An active web3js connection.
 * @param stakePoolProgramAddress: The public key (address) of the StakePool program.
 */
export async function getStakePoolAccounts(
  connection: Connection,
  stakePoolProgramAddress: PublicKey,
): Promise<(StakePoolAccount | ValidatorListAccount)[] | undefined> {
  const response = await connection.getProgramAccounts(stakePoolProgramAddress);

  return response.map(a => {
    let decodedData;

    if (a.account.data.readUInt8() === 1) {
      try {
        decodedData = STAKE_POOL_LAYOUT.decode(a.account.data);
      } catch (error) {
        console.log('Could not decode StakeAccount. Error:', error);
        decodedData = undefined;
      }
    } else if (a.account.data.readUInt8() === 2) {
      try {
        decodedData = VALIDATOR_LIST_LAYOUT.decode(a.account.data);
      } catch (error) {
        console.log('Could not decode ValidatorList. Error:', error);
        decodedData = undefined;
      }
    } else {
      console.error(
        `Could not decode. StakePoolAccount Enum is ${a.account.data.readUInt8()}, expected 1 or 2!`,
      );
      decodedData = undefined;
    }

    return {
      pubkey: a.pubkey,
      account: {
        data: decodedData,
        executable: a.account.executable,
        lamports: a.account.lamports,
        owner: a.account.owner,
      },
    };
  });
}

/**
 * Helper function to pretty print a schema.PublicKey
 * Pretty prints a PublicKey in base58 format */
export function prettyPrintPubKey(pubKey: PublicKey): string {
  return new PublicKey(new PublicKey(pubKey.toBuffer()).toBytes().reverse()).toString();
}

/**
 * Helper function to pretty print a decoded account
 */
export function prettyPrintAccount(account: ValidatorListAccount | StakePoolAccount): void {
  console.log('Address:', account.pubkey.toString());
  const sp = account.account.data;
  if (typeof sp === 'undefined') {
    console.log('Account could not be decoded');
    return;
  }
  for (const val in sp) {
    // @ts-ignore
    if (sp[val] instanceof PublicKey) {
      // @ts-ignore
      console.log(val, prettyPrintPubKey(sp[val]));
    } else {
      // @ts-ignore
      console.log(val, sp[val]);
    }
  }
  console.log('Executable?:', account.account.executable);
  console.log('Lamports:', account.account.lamports);
  console.log('Owner PubKey:', account.account.owner.toString());
}

/**
 * Creates instructions required to deposit sol to stake pool.
 */
export async function depositSol(
  connection: Connection,
  stakePoolAddress: PublicKey,
  from: PublicKey,
  lamports: number,
  poolTokenReceiverAccount?: PublicKey,
  referrerTokenAccount?: PublicKey,
) {
  const fromBalance = await connection.getBalance(from, 'confirmed');
  if (fromBalance < lamports) {
    throw new Error(
      `Not enough SOL to deposit into pool. Maximum deposit amount is ${lamportsToSol(
        fromBalance,
      )} SOL.`,
    );
  }

  const stakePoolAccount = await getStakePoolAccount(connection, stakePoolAddress);
  const stakePool = stakePoolAccount.account.data;

  // Ephemeral SOL account just to do the transfer
  const userSolTransfer = new Keypair();
  const signers: Signer[] = [userSolTransfer];
  const instructions: TransactionInstruction[] = [];

  // Create the ephemeral SOL account
  instructions.push(
    SystemProgram.transfer({
      fromPubkey: from,
      toPubkey: userSolTransfer.publicKey,
      lamports,
    }),
  );

  const { poolMint } = stakePool;

  // Create token account if not specified
  if (!poolTokenReceiverAccount) {
    poolTokenReceiverAccount = await addAssociatedTokenAccount(
      connection,
      from,
      poolMint,
      instructions,
    );
  }

  const depositAuthority = undefined;

  const withdrawAuthority = await findWithdrawAuthorityProgramAddress(
    StakePoolProgram.programId,
    stakePoolAddress,
  );

  instructions.push(
    StakePoolProgram.depositSolInstruction({
      stakePoolPubkey: stakePoolAddress,
      depositAuthority,
      withdrawAuthority,
      reserveStakeAccount: stakePool.reserveStake,
      lamportsFrom: userSolTransfer.publicKey,
      poolTokensTo: poolTokenReceiverAccount,
      managerFeeAccount: stakePool.managerFeeAccount,
      referrerPoolTokensAccount: referrerTokenAccount ?? poolTokenReceiverAccount,
      poolMint,
      lamports,
    }),
  );

  return {
    instructions,
    signers,
  };
}

/**
 * Creates instructions required to withdraw stake from a stake pool.
 */
export async function withdrawStake(
  connection: Connection,
  stakePoolProgramAddress: PublicKey,
  tokenOwner: PublicKey,
  amount: number,
  useReserve = false,
  voteAccountAddress?: PublicKey,
  stakeReceiver?: PublicKey,
  poolTokenAccount?: PublicKey,
) {
  const stakePool = await getStakePoolAccount(connection, stakePoolProgramAddress);
  const poolAmount = solToLamports(amount);

  if (!poolTokenAccount) {
    poolTokenAccount = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      stakePool.account.data.poolMint,
      tokenOwner,
    );
  }

  const tokenAccount = await getTokenAccount(
    connection,
    poolTokenAccount,
    stakePool.account.data.poolMint,
  );
  if (!tokenAccount) {
    throw new Error('Invalid token account');
  }

  // Check withdrawFrom balance
  if (tokenAccount.amount.toNumber() < poolAmount) {
    throw new Error(
      `Not enough token balance to withdraw ${lamportsToSol(poolAmount)} pool tokens.
          Maximum withdraw amount is ${lamportsToSol(tokenAccount.amount.toNumber())} pool tokens.`,
    );
  }

  const poolWithdrawAuthority = await findWithdrawAuthorityProgramAddress(
    StakePoolProgram.programId,
    stakePoolProgramAddress,
  );

  const withdrawAccounts: WithdrawAccount[] = [];

  if (useReserve) {
    withdrawAccounts.push({
      stakeAddress: stakePool.account.data.reserveStake,
      voteAddress: undefined,
      poolAmount,
    });
  } else if (voteAccountAddress) {
    const stakeAccountAddress = await findStakeProgramAddress(
      StakePoolProgram.programId,
      voteAccountAddress,
      stakePoolProgramAddress,
    );
    const stakeAccount = await connection.getAccountInfo(stakeAccountAddress);
    if (!stakeAccount) {
      throw new Error('Invalid Stake Account');
    }

    const availableForWithdrawal = calcLamportsWithdrawAmount(
      stakePool.account.data,
      stakeAccount.lamports - MIN_STAKE_BALANCE,
    );

    if (availableForWithdrawal < poolAmount) {
      // noinspection ExceptionCaughtLocallyJS
      throw new Error(
        `Not enough lamports available for withdrawal from ${stakeAccountAddress},
                ${poolAmount} asked, ${availableForWithdrawal} available.`,
      );
    }
    withdrawAccounts.push({
      stakeAddress: stakeAccountAddress,
      voteAddress: voteAccountAddress,
      poolAmount,
    });
  } else {
    // Get the list of accounts to withdraw from
    withdrawAccounts.push(
      ...(await prepareWithdrawAccounts(
        connection,
        stakePool.account.data,
        stakePoolProgramAddress,
        poolAmount,
      )),
    );
  }

  // Construct transaction to withdraw from withdrawAccounts account list
  const instructions: TransactionInstruction[] = [];
  const userTransferAuthority = Keypair.generate();

  const signers: Signer[] = [userTransferAuthority];

  instructions.push(
    Token.createApproveInstruction(
      TOKEN_PROGRAM_ID,
      poolTokenAccount,
      userTransferAuthority.publicKey,
      tokenOwner,
      [],
      poolAmount,
    ),
  );

  let totalRentFreeBalances = 0;

  // Go through prepared accounts and withdraw/claim them
  for (const withdrawAccount of withdrawAccounts) {
    // Convert pool tokens amount to lamports
    const solWithdrawAmount = Math.ceil(
      calcLamportsWithdrawAmount(stakePool.account.data, withdrawAccount.poolAmount),
    );

    let infoMsg = `Withdrawing ◎${solWithdrawAmount},
        or ${amountToUiAmount(withdrawAccount.poolAmount, 9)} pool tokens,
        from stake account ${withdrawAccount.stakeAddress?.toBase58()}`;

    if (withdrawAccount.voteAddress) {
      infoMsg = `${infoMsg}, delegated to ${withdrawAccount.voteAddress?.toBase58()}`;
    }

    console.info(infoMsg);

    let stakeToReceive;

    // Use separate mutable variable because withdraw might create a new account
    if (!stakeReceiver) {
      const stakeReceiverAccountBalance = await connection.getMinimumBalanceForRentExemption(
        STAKE_STATE_LEN,
      );
      const stakeKeypair = newStakeAccount(tokenOwner, instructions, stakeReceiverAccountBalance);
      signers.push(stakeKeypair);
      totalRentFreeBalances += stakeReceiverAccountBalance;
      stakeToReceive = stakeKeypair.publicKey;
    } else {
      stakeToReceive = stakeReceiver;
    }

    // console.info(`Stake to Split ${withdrawAccount.stakeAddress.toBase58()}`);
    // console.info(`Stake to Receive ${stakeToReceive.toBase58()}`);
    // console.info(`Pool Withdraw Authority ${poolWithdrawAuthority.toBase58()}`);
    // console.info(`Manager Fee Account ${stakePool.account.data.managerFeeAccount.toBase58()}`);
    // console.info(`Pool Mint ${stakePool.account.data.poolMint.toBase58()}`);
    // console.info(`Pool Amount ${withdrawAccount.poolAmount}`);
    // console.info(`Total Rent Free Balances ${totalRentFreeBalances}`);

    const withdrawTransaction = StakePoolProgram.withdrawStakeInstruction({
      stakePoolPubkey: stakePoolProgramAddress,
      validatorListStorage: stakePool.account.data.validatorList,
      stakePoolWithdrawAuthority: poolWithdrawAuthority,
      stakeToSplit: withdrawAccount.stakeAddress,
      stakeToReceive,
      userStakeAuthority: tokenOwner,
      userTransferAuthority: userTransferAuthority.publicKey,
      userPoolTokenAccount: poolTokenAccount,
      managerFeeAccount: stakePool.account.data.managerFeeAccount,
      poolMint: stakePool.account.data.poolMint,
      lamports: withdrawAccount.poolAmount,
    });

    instructions.push(withdrawTransaction);
  }

  return {
    instructions,
    signers,
    stakeReceiver,
    totalRentFreeBalances,
  };
}

/**
 * Creates instructions required to withdraw SOL directly from a stake pool.
 */
export async function withdrawSol(
  connection: Connection,
  stakePoolAddress: PublicKey,
  tokenOwner: PublicKey,
  solReceiver: PublicKey,
  amount: number,
  solWithdrawAuthority?: PublicKey,
) {
  const stakePool = await getStakePoolAccount(connection, stakePoolAddress);
  const poolAmount = solToLamports(amount);

  const poolTokenAccount = await Token.getAssociatedTokenAddress(
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    stakePool.account.data.poolMint,
    tokenOwner,
  );

  const tokenAccount = await getTokenAccount(
    connection,
    poolTokenAccount,
    stakePool.account.data.poolMint,
  );
  if (!tokenAccount) {
    throw new Error('Invalid token account');
  }

  // Check withdrawFrom balance
  if (tokenAccount.amount.toNumber() < poolAmount) {
    throw new Error(
      `Not enough token balance to withdraw ${lamportsToSol(poolAmount)} pool tokens.
          Maximum withdraw amount is ${lamportsToSol(tokenAccount.amount.toNumber())} pool tokens.`,
    );
  }

  // Construct transaction to withdraw from withdrawAccounts account list
  const instructions: TransactionInstruction[] = [];
  const userTransferAuthority = Keypair.generate();
  const signers: Signer[] = [userTransferAuthority];

  instructions.push(
    Token.createApproveInstruction(
      TOKEN_PROGRAM_ID,
      poolTokenAccount,
      userTransferAuthority.publicKey,
      tokenOwner,
      [],
      poolAmount,
    ),
  );

  const poolWithdrawAuthority = await findWithdrawAuthorityProgramAddress(
    StakePoolProgram.programId,
    stakePoolAddress,
  );

  if (solWithdrawAuthority) {
    const expectedSolWithdrawAuthority = stakePool.account.data.solWithdrawAuthority;
    if (!expectedSolWithdrawAuthority) {
      throw new Error('SOL withdraw authority specified in arguments but stake pool has none');
    }
    if (solWithdrawAuthority.toBase58() != expectedSolWithdrawAuthority.toBase58()) {
      throw new Error(
        `Invalid deposit withdraw specified, expected ${expectedSolWithdrawAuthority.toBase58()}, received ${solWithdrawAuthority.toBase58()}`,
      );
    }
  }

  const withdrawTransaction = StakePoolProgram.withdrawSolInstruction({
    stakePoolPubkey: stakePoolAddress,
    solWithdrawAuthority: solWithdrawAuthority,
    stakePoolWithdrawAuthority: poolWithdrawAuthority,
    userTransferAuthority: userTransferAuthority.publicKey,
    poolTokensFrom: poolTokenAccount,
    reserveStakeAccount: stakePool.account.data.reserveStake,
    managerFeeAccount: stakePool.account.data.managerFeeAccount,
    poolMint: stakePool.account.data.poolMint,
    lamportsTo: solReceiver,
    poolTokens: poolAmount,
  });

  instructions.push(withdrawTransaction);

  return {
    instructions,
    signers,
  };
}
