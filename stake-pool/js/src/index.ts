import {
  AccountInfo,
  Connection,
  Keypair,
  PublicKey,
  Signer,
  StakeAuthorizationLayout,
  StakeProgram,
  SystemProgram,
  TransactionInstruction,
} from '@solana/web3.js';
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, Token } from '@solana/spl-token';
import {
  ValidatorAccount,
  addAssociatedTokenAccount,
  arrayChunk,
  calcLamportsWithdrawAmount,
  findStakeProgramAddress,
  findTransientStakeProgramAddress,
  findWithdrawAuthorityProgramAddress,
  getTokenAccount,
  getValidatorListAccount,
  newStakeAccount,
  prepareWithdrawAccounts,
  lamportsToSol,
  solToLamports,
} from './utils';
import { StakePoolInstruction } from './instructions';
import {
  StakePool,
  StakePoolLayout,
  ValidatorList,
  ValidatorListLayout,
  ValidatorStakeInfo,
} from './layouts';
import { MAX_VALIDATORS_TO_UPDATE, MINIMUM_ACTIVE_STAKE, STAKE_POOL_PROGRAM_ID } from './constants';

export type { StakePool, AccountType, ValidatorList, ValidatorStakeInfo } from './layouts';
export { STAKE_POOL_PROGRAM_ID } from './constants';
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
 * @param stakePoolAddress: The public key (address) of the stake pool account.
 */
export async function getStakePoolAccount(
  connection: Connection,
  stakePoolAddress: PublicKey,
): Promise<StakePoolAccount> {
  const account = await connection.getAccountInfo(stakePoolAddress);

  if (!account) {
    throw new Error('Invalid stake pool account');
  }

  return {
    pubkey: stakePoolAddress,
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

  return response.map((a) => {
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
 * Creates instructions required to deposit stake to stake pool.
 */
export async function depositStake(
  connection: Connection,
  stakePoolAddress: PublicKey,
  authorizedPubkey: PublicKey,
  validatorVote: PublicKey,
  depositStake: PublicKey,
  poolTokenReceiverAccount?: PublicKey,
) {
  const stakePool = await getStakePoolAccount(connection, stakePoolAddress);

  const withdrawAuthority = await findWithdrawAuthorityProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    stakePoolAddress,
  );

  const validatorStake = await findStakeProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    validatorVote,
    stakePoolAddress,
  );

  const instructions: TransactionInstruction[] = [];
  const signers: Signer[] = [];

  const poolMint = stakePool.account.data.poolMint;

  let rentFee = 0;

  // Create token account if not specified
  if (!poolTokenReceiverAccount) {
    const { associatedAddress, rentFee: fee } = await addAssociatedTokenAccount(
      connection,
      authorizedPubkey,
      poolMint,
      instructions,
    );
    poolTokenReceiverAccount = associatedAddress;
    rentFee += fee;
  }

  instructions.push(
    ...StakeProgram.authorize({
      stakePubkey: depositStake,
      authorizedPubkey,
      newAuthorizedPubkey: stakePool.account.data.stakeDepositAuthority,
      stakeAuthorizationType: StakeAuthorizationLayout.Staker,
    }).instructions,
  );

  instructions.push(
    ...StakeProgram.authorize({
      stakePubkey: depositStake,
      authorizedPubkey,
      newAuthorizedPubkey: stakePool.account.data.stakeDepositAuthority,
      stakeAuthorizationType: StakeAuthorizationLayout.Withdrawer,
    }).instructions,
  );

  instructions.push(
    StakePoolInstruction.depositStake({
      stakePool: stakePoolAddress,
      validatorList: stakePool.account.data.validatorList,
      depositAuthority: stakePool.account.data.stakeDepositAuthority,
      reserveStake: stakePool.account.data.reserveStake,
      managerFeeAccount: stakePool.account.data.managerFeeAccount,
      referralPoolAccount: poolTokenReceiverAccount,
      destinationPoolAccount: poolTokenReceiverAccount,
      withdrawAuthority,
      depositStake,
      validatorStake,
      poolMint,
    }),
  );

  return {
    instructions,
    signers,
    rentFee,
  };
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

  const stakePoolAccount = await getStakePoolAccount(connection, stakePoolAddress);
  const stakePool = stakePoolAccount.account.data;

  // Ephemeral SOL account just to do the transfer
  const userSolTransfer = new Keypair();
  const signers: Signer[] = [userSolTransfer];
  const instructions: TransactionInstruction[] = [];

  let rentFee = 0;

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
    const { associatedAddress, rentFee: fee } = await addAssociatedTokenAccount(
      connection,
      from,
      stakePool.poolMint,
      instructions,
    );
    destinationTokenAccount = associatedAddress;
    rentFee += fee;
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
      lamports,
      withdrawAuthority,
      depositAuthority,
    }),
  );

  return {
    instructions,
    signers,
    rentFee,
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
  validatorComparator?: (_a: ValidatorAccount, _b: ValidatorAccount) => number,
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
      `Not enough token balance to withdraw ${lamportsToSol(poolAmount)} pool tokens.
        Maximum withdraw amount is ${lamportsToSol(tokenAccount.amount.toNumber())} pool tokens.`,
    );
  }

  const stakeAccountRentExemption = await connection.getMinimumBalanceForRentExemption(
    StakeProgram.space,
  );

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
      stakeAccount.lamports - MINIMUM_ACTIVE_STAKE - stakeAccountRentExemption,
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
        validatorComparator,
        poolTokenAccount.equals(stakePool.account.data.managerFeeAccount),
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
      calcLamportsWithdrawAmount(stakePool.account.data, withdrawAccount.poolAmount),
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
      const stakeKeypair = newStakeAccount(tokenOwner, instructions, stakeAccountRentExemption);
      signers.push(stakeKeypair);
      totalRentFreeBalances += stakeAccountRentExemption;
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
    STAKE_POOL_PROGRAM_ID,
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

/**
 * Creates instructions required to increase validator stake.
 */
export async function increaseValidatorStake(
  connection: Connection,
  stakePoolAddress: PublicKey,
  validatorVote: PublicKey,
  lamports: number,
) {
  const stakePool = await getStakePoolAccount(connection, stakePoolAddress);

  const validatorList = await getValidatorListAccount(
    connection,
    stakePool.account.data.validatorList,
  );

  const validatorInfo = validatorList.account.data.validators.find(
    (v) => v.voteAccountAddress.toBase58() == validatorVote.toBase58(),
  );

  if (!validatorInfo) {
    throw new Error('Vote account not found in validator list');
  }

  const withdrawAuthority = await findWithdrawAuthorityProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    stakePoolAddress,
  );

  const transientStakeSeed = validatorInfo.transientSeedSuffixStart.addn(1); // bump up by one to avoid reuse

  const transientStake = await findTransientStakeProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    validatorInfo.voteAccountAddress,
    stakePoolAddress,
    transientStakeSeed,
  );

  const instructions: TransactionInstruction[] = [];
  instructions.push(
    StakePoolInstruction.increaseValidatorStake({
      stakePool: stakePoolAddress,
      staker: stakePool.account.data.staker,
      validatorList: stakePool.account.data.validatorList,
      reserveStake: stakePool.account.data.reserveStake,
      transientStakeSeed: transientStakeSeed.toNumber(),
      withdrawAuthority,
      transientStake,
      validatorVote,
      lamports,
    }),
  );

  return {
    instructions,
  };
}

/**
 * Creates instructions required to decrease validator stake.
 */
export async function decreaseValidatorStake(
  connection: Connection,
  stakePoolAddress: PublicKey,
  validatorVote: PublicKey,
  lamports: number,
) {
  const stakePool = await getStakePoolAccount(connection, stakePoolAddress);
  const validatorList = await getValidatorListAccount(
    connection,
    stakePool.account.data.validatorList,
  );

  const validatorInfo = validatorList.account.data.validators.find(
    (v) => v.voteAccountAddress.toBase58() == validatorVote.toBase58(),
  );

  if (!validatorInfo) {
    throw new Error('Vote account not found in validator list');
  }

  const withdrawAuthority = await findWithdrawAuthorityProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    stakePoolAddress,
  );

  const validatorStake = await findStakeProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    validatorInfo.voteAccountAddress,
    stakePoolAddress,
  );

  const transientStakeSeed = validatorInfo.transientSeedSuffixStart.addn(1); // bump up by one to avoid reuse

  const transientStake = await findTransientStakeProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    validatorInfo.voteAccountAddress,
    stakePoolAddress,
    transientStakeSeed,
  );

  const instructions: TransactionInstruction[] = [];
  instructions.push(
    StakePoolInstruction.decreaseValidatorStake({
      stakePool: stakePoolAddress,
      staker: stakePool.account.data.staker,
      validatorList: stakePool.account.data.validatorList,
      transientStakeSeed: transientStakeSeed.toNumber(),
      withdrawAuthority,
      validatorStake,
      transientStake,
      lamports,
    }),
  );

  return {
    instructions,
  };
}

/**
 * Creates instructions required to completely update a stake pool after epoch change.
 */
export async function updateStakePool(
  connection: Connection,
  stakePool: StakePoolAccount,
  noMerge = false,
) {
  const stakePoolAddress = stakePool.pubkey;

  const validatorList = await getValidatorListAccount(
    connection,
    stakePool.account.data.validatorList,
  );

  const withdrawAuthority = await findWithdrawAuthorityProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    stakePoolAddress,
  );

  const updateListInstructions: TransactionInstruction[] = [];
  const instructions: TransactionInstruction[] = [];

  let startIndex = 0;
  const validatorChunks: Array<ValidatorStakeInfo[]> = arrayChunk(
    validatorList.account.data.validators,
    MAX_VALIDATORS_TO_UPDATE,
  );

  for (const validatorChunk of validatorChunks) {
    const validatorAndTransientStakePairs: PublicKey[] = [];

    for (const validator of validatorChunk) {
      const validatorStake = await findStakeProgramAddress(
        STAKE_POOL_PROGRAM_ID,
        validator.voteAccountAddress,
        stakePoolAddress,
      );
      validatorAndTransientStakePairs.push(validatorStake);

      const transientStake = await findTransientStakeProgramAddress(
        STAKE_POOL_PROGRAM_ID,
        validator.voteAccountAddress,
        stakePoolAddress,
        validator.transientSeedSuffixStart,
      );
      validatorAndTransientStakePairs.push(transientStake);
    }

    updateListInstructions.push(
      StakePoolInstruction.updateValidatorListBalance({
        stakePool: stakePoolAddress,
        validatorList: stakePool.account.data.validatorList,
        reserveStake: stakePool.account.data.reserveStake,
        validatorAndTransientStakePairs,
        withdrawAuthority,
        startIndex,
        noMerge,
      }),
    );
    startIndex += MAX_VALIDATORS_TO_UPDATE;
  }

  instructions.push(
    StakePoolInstruction.updateStakePoolBalance({
      stakePool: stakePoolAddress,
      validatorList: stakePool.account.data.validatorList,
      reserveStake: stakePool.account.data.reserveStake,
      managerFeeAccount: stakePool.account.data.managerFeeAccount,
      poolMint: stakePool.account.data.poolMint,
      withdrawAuthority,
    }),
  );

  instructions.push(
    StakePoolInstruction.cleanupRemovedValidatorEntries({
      stakePool: stakePoolAddress,
      validatorList: stakePool.account.data.validatorList,
    }),
  );

  return {
    updateListInstructions,
    finalInstructions: instructions,
  };
}

/**
 * Retrieves detailed information about the StakePool.
 */
export async function stakePoolInfo(connection: Connection, stakePoolAddress: PublicKey) {
  const stakePool = await getStakePoolAccount(connection, stakePoolAddress);
  const reserveAccountStakeAddress = stakePool.account.data.reserveStake;
  const totalLamports = stakePool.account.data.totalLamports;
  const lastUpdateEpoch = stakePool.account.data.lastUpdateEpoch;

  const validatorList = await getValidatorListAccount(
    connection,
    stakePool.account.data.validatorList,
  );

  const maxNumberOfValidators = validatorList.account.data.maxValidators;
  const currentNumberOfValidators = validatorList.account.data.validators.length;

  const epochInfo = await connection.getEpochInfo();
  const reserveStake = await connection.getAccountInfo(reserveAccountStakeAddress);
  const withdrawAuthority = await findWithdrawAuthorityProgramAddress(
    STAKE_POOL_PROGRAM_ID,
    stakePoolAddress,
  );

  const minimumReserveStakeBalance =
    (await connection.getMinimumBalanceForRentExemption(StakeProgram.space)) + 1;

  const stakeAccounts = await Promise.all(
    validatorList.account.data.validators.map(async (validator) => {
      const stakeAccountAddress = await findStakeProgramAddress(
        STAKE_POOL_PROGRAM_ID,
        validator.voteAccountAddress,
        stakePoolAddress,
      );
      const transientStakeAccountAddress = await findTransientStakeProgramAddress(
        STAKE_POOL_PROGRAM_ID,
        validator.voteAccountAddress,
        stakePoolAddress,
        validator.transientSeedSuffixStart,
      );
      const updateRequired = !validator.lastUpdateEpoch.eqn(epochInfo.epoch);
      return {
        voteAccountAddress: validator.voteAccountAddress.toBase58(),
        stakeAccountAddress: stakeAccountAddress.toBase58(),
        validatorActiveStakeLamports: validator.activeStakeLamports.toString(),
        validatorLastUpdateEpoch: validator.lastUpdateEpoch.toString(),
        validatorLamports: validator.activeStakeLamports
          .add(validator.transientStakeLamports)
          .toString(),
        validatorTransientStakeAccountAddress: transientStakeAccountAddress.toBase58(),
        validatorTransientStakeLamports: validator.transientStakeLamports.toString(),
        updateRequired,
      };
    }),
  );

  const totalPoolTokens = lamportsToSol(stakePool.account.data.poolTokenSupply);
  const updateRequired = !lastUpdateEpoch.eqn(epochInfo.epoch);

  return {
    address: stakePoolAddress.toBase58(),
    poolWithdrawAuthority: withdrawAuthority.toBase58(),
    manager: stakePool.account.data.manager.toBase58(),
    staker: stakePool.account.data.staker.toBase58(),
    stakeDepositAuthority: stakePool.account.data.stakeDepositAuthority.toBase58(),
    stakeWithdrawBumpSeed: stakePool.account.data.stakeWithdrawBumpSeed,
    maxValidators: maxNumberOfValidators,
    validatorList: validatorList.account.data.validators.map((validator) => {
      return {
        activeStakeLamports: validator.activeStakeLamports.toString(),
        transientStakeLamports: validator.transientStakeLamports.toString(),
        lastUpdateEpoch: validator.lastUpdateEpoch.toString(),
        transientSeedSuffixStart: validator.transientSeedSuffixStart.toString(),
        transientSeedSuffixEnd: validator.transientSeedSuffixEnd.toString(),
        status: validator.status.toString(),
        voteAccountAddress: validator.voteAccountAddress.toString(),
      };
    }), // CliStakePoolValidator
    validatorListStorageAccount: stakePool.account.data.validatorList.toBase58(),
    reserveStake: stakePool.account.data.reserveStake.toBase58(),
    poolMint: stakePool.account.data.poolMint.toBase58(),
    managerFeeAccount: stakePool.account.data.managerFeeAccount.toBase58(),
    tokenProgramId: stakePool.account.data.tokenProgramId.toBase58(),
    totalLamports: stakePool.account.data.totalLamports.toString(),
    poolTokenSupply: stakePool.account.data.poolTokenSupply.toString(),
    lastUpdateEpoch: stakePool.account.data.lastUpdateEpoch.toString(),
    lockup: stakePool.account.data.lockup, // pub lockup: CliStakePoolLockup
    epochFee: stakePool.account.data.epochFee,
    nextEpochFee: stakePool.account.data.nextEpochFee,
    preferredDepositValidatorVoteAddress:
      stakePool.account.data.preferredDepositValidatorVoteAddress,
    preferredWithdrawValidatorVoteAddress:
      stakePool.account.data.preferredWithdrawValidatorVoteAddress,
    stakeDepositFee: stakePool.account.data.stakeDepositFee,
    stakeWithdrawalFee: stakePool.account.data.stakeWithdrawalFee,
    // CliStakePool the same
    nextStakeWithdrawalFee: stakePool.account.data.nextStakeWithdrawalFee,
    stakeReferralFee: stakePool.account.data.stakeReferralFee,
    solDepositAuthority: stakePool.account.data.solDepositAuthority?.toBase58(),
    solDepositFee: stakePool.account.data.solDepositFee,
    solReferralFee: stakePool.account.data.solReferralFee,
    solWithdrawAuthority: stakePool.account.data.solWithdrawAuthority?.toBase58(),
    solWithdrawalFee: stakePool.account.data.solWithdrawalFee,
    nextSolWithdrawalFee: stakePool.account.data.nextSolWithdrawalFee,
    lastEpochPoolTokenSupply: stakePool.account.data.lastEpochPoolTokenSupply.toString(),
    lastEpochTotalLamports: stakePool.account.data.lastEpochTotalLamports.toString(),
    details: {
      reserveStakeLamports: reserveStake?.lamports,
      reserveAccountStakeAddress: reserveAccountStakeAddress.toBase58(),
      minimumReserveStakeBalance,
      stakeAccounts,
      totalLamports,
      totalPoolTokens,
      currentNumberOfValidators,
      maxNumberOfValidators,
      updateRequired,
    }, // CliStakePoolDetails
  };
}
