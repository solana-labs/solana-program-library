import {
  AccountInfo,
  Connection,
  Keypair,
  PublicKey,
  Signer,
  StakeProgram,
  SystemProgram,
  TransactionInstruction,
} from '@solana/web3.js';
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  Token,
} from '@solana/spl-token';
import {
  addAssociatedTokenAccount,
  calcLamportsWithdrawAmount,
  findStakeProgramAddress,
  findWithdrawAuthorityProgramAddress,
  getTokenAccount,
  newStakeAccount,
  prepareWithdrawAccounts,
  lamportsToSol,
  solToLamports,
} from './utils';
import {StakePoolInstruction} from './instructions';
import {
  StakePoolLayout,
  ValidatorListLayout,
  ValidatorList,
  StakePool,
} from './layouts';
import {MIN_STAKE_BALANCE, STAKE_POOL_PROGRAM_ID} from './constants';

export type {
  StakePool,
  AccountType,
  ValidatorList,
  ValidatorStakeInfo,
} from './layouts';
export {STAKE_POOL_PROGRAM_ID} from './constants';
export * from './instructions';

export interface ValidatorListAccount {
  pubkey: PublicKey;
  account: AccountInfo<ValidatorList>;
}

export interface StakePoolAccount {
  pubkey: PublicKey;
  account: AccountInfo<StakePool>;
}

export interface WithdrawAccount {
  stakeAddress: PublicKey;
  voteAddress?: PublicKey;
  poolAmount: number;
}

/**
 * Wrapper class for a stake pool.
 * Each stake pool has a stake pool account and a validator list account.
 */
export interface StakePoolAccounts {
  stakePool: StakePoolAccount | undefined;
  validatorList: ValidatorListAccount | undefined;
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
      data: StakePoolLayout.decode(account.data),
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
        decodedData = StakePoolLayout.decode(a.account.data);
      } catch (error) {
        console.log('Could not decode StakeAccount. Error:', error);
        decodedData = undefined;
      }
    } else if (a.account.data.readUInt8() === 2) {
      try {
        decodedData = ValidatorListLayout.decode(a.account.data);
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
 * Creates instructions required to deposit sol to stake pool.
 */
export async function depositSol(
  connection: Connection,
  stakePoolAddress: PublicKey,
  from: PublicKey,
  lamports: number,
  destinationTokenAccount?: PublicKey,
  referrerTokenAccount?: PublicKey,
  depositAuthority?: PublicKey,
) {
  const fromBalance = await connection.getBalance(from, 'confirmed');
  if (fromBalance < lamports) {
    throw new Error(
      `Not enough SOL to deposit into pool. Maximum deposit amount is ${lamportsToSol(
        fromBalance,
      )} SOL.`,
    );
  }

  const stakePoolAccount = await getStakePoolAccount(
    connection,
    stakePoolAddress,
  );
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

  // Create token account if not specified
  if (!destinationTokenAccount) {
    destinationTokenAccount = await addAssociatedTokenAccount(
      connection,
      from,
      stakePool.poolMint,
      instructions,
    );
  }

  const withdrawAuthority = await findWithdrawAuthorityProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    stakePoolAddress,
  );

  instructions.push(
    StakePoolInstruction.depositSol({
      stakePool: stakePoolAddress,
      reserveStake: stakePool.reserveStake,
      fundingAccount: userSolTransfer.publicKey,
      destinationPoolAccount: destinationTokenAccount,
      managerFeeAccount: stakePool.managerFeeAccount,
      referralPoolAccount: referrerTokenAccount ?? destinationTokenAccount,
      poolMint: stakePool.poolMint,
      lamports: lamports,
      withdrawAuthority: withdrawAuthority,
      depositAuthority: depositAuthority,
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
  stakePoolAddress: PublicKey,
  tokenOwner: PublicKey,
  amount: number,
  useReserve = false,
  voteAccountAddress?: PublicKey,
  stakeReceiver?: PublicKey,
  poolTokenAccount?: PublicKey,
) {
  const stakePool = await getStakePoolAccount(connection, stakePoolAddress);
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
      `Not enough token balance to withdraw ${lamportsToSol(
        poolAmount,
      )} pool tokens.
        Maximum withdraw amount is ${lamportsToSol(
          tokenAccount.amount.toNumber(),
        )} pool tokens.`,
    );
  }

  const withdrawAuthority = await findWithdrawAuthorityProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    stakePoolAddress,
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
      STAKE_POOL_PROGRAM_ID,
      voteAccountAddress,
      stakePoolAddress,
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
        stakePoolAddress,
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

  // Max 5 accounts to prevent an error: "Transaction too large"
  const maxWithdrawAccounts = 5;
  let i = 0;

  // Go through prepared accounts and withdraw/claim them
  for (const withdrawAccount of withdrawAccounts) {
    if (i > maxWithdrawAccounts) {
      break;
    }
    // Convert pool tokens amount to lamports
    const solWithdrawAmount = Math.ceil(
      calcLamportsWithdrawAmount(
        stakePool.account.data,
        withdrawAccount.poolAmount,
      ),
    );

    let infoMsg = `Withdrawing ◎${solWithdrawAmount},
      from stake account ${withdrawAccount.stakeAddress?.toBase58()}`;

    if (withdrawAccount.voteAddress) {
      infoMsg = `${infoMsg}, delegated to ${withdrawAccount.voteAddress?.toBase58()}`;
    }

    console.info(infoMsg);

    let stakeToReceive;

    // Use separate mutable variable because withdraw might create a new account
    if (!stakeReceiver) {
      const stakeReceiverAccountBalance =
        await connection.getMinimumBalanceForRentExemption(StakeProgram.space);
      const stakeKeypair = newStakeAccount(
        tokenOwner,
        instructions,
        stakeReceiverAccountBalance,
      );
      signers.push(stakeKeypair);
      totalRentFreeBalances += stakeReceiverAccountBalance;
      stakeToReceive = stakeKeypair.publicKey;
    } else {
      stakeToReceive = stakeReceiver;
    }

    instructions.push(
      StakePoolInstruction.withdrawStake({
        stakePool: stakePoolAddress,
        validatorList: stakePool.account.data.validatorList,
        validatorStake: withdrawAccount.stakeAddress,
        destinationStake: stakeToReceive,
        destinationStakeAuthority: tokenOwner,
        sourceTransferAuthority: userTransferAuthority.publicKey,
        sourcePoolAccount: poolTokenAccount,
        managerFeeAccount: stakePool.account.data.managerFeeAccount,
        poolMint: stakePool.account.data.poolMint,
        poolTokens: withdrawAccount.poolAmount,
        withdrawAuthority,
      }),
    );
    i++;
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
      `Not enough token balance to withdraw ${lamportsToSol(
        poolAmount,
      )} pool tokens.
          Maximum withdraw amount is ${lamportsToSol(
            tokenAccount.amount.toNumber(),
          )} pool tokens.`,
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
    STAKE_POOL_PROGRAM_ID,
    stakePoolAddress,
  );

  if (solWithdrawAuthority) {
    const expectedSolWithdrawAuthority =
      stakePool.account.data.solWithdrawAuthority;
    if (!expectedSolWithdrawAuthority) {
      throw new Error(
        'SOL withdraw authority specified in arguments but stake pool has none',
      );
    }
    if (
      solWithdrawAuthority.toBase58() != expectedSolWithdrawAuthority.toBase58()
    ) {
      throw new Error(
        `Invalid deposit withdraw specified, expected ${expectedSolWithdrawAuthority.toBase58()}, received ${solWithdrawAuthority.toBase58()}`,
      );
    }
  }

  const withdrawTransaction = StakePoolInstruction.withdrawSol({
    stakePool: stakePoolAddress,
    withdrawAuthority: poolWithdrawAuthority,
    reserveStake: stakePool.account.data.reserveStake,
    sourcePoolAccount: poolTokenAccount,
    sourceTransferAuthority: userTransferAuthority.publicKey,
    destinationSystemAccount: solReceiver,
    managerFeeAccount: stakePool.account.data.managerFeeAccount,
    poolMint: stakePool.account.data.poolMint,
    poolTokens: poolAmount,
    solWithdrawAuthority,
  });

  instructions.push(withdrawTransaction);

  return {
    instructions,
    signers,
  };
}
