import {
  LAMPORTS_PER_SOL,
  PublicKey,
  Connection,
  TransactionInstruction,
  Keypair,
  StakeProgram,
  SystemProgram,
} from '@solana/web3.js';
import {
  MintInfo,
  Token,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  AccountInfo
} from '@solana/spl-token';
import { Buffer } from 'buffer';
import BN from 'bn.js';
import { ValidatorListLayout, AccountLayout, ValidatorList, StakePool } from './layouts';

import { STAKE_STATE_LEN, StakePoolProgram, WithdrawAccount } from './stakepool-program';

const TRANSIENT_STAKE_SEED_PREFIX = Buffer.from('transient');
const FAILED_TO_FIND_ACCOUNT = 'Failed to find account';
const INVALID_ACCOUNT_OWNER = 'Invalid account owner';

export function solToLamports(amount: number): number {
  if (isNaN(amount)) return Number(0);
  return Number(amount * LAMPORTS_PER_SOL);
}

export function lamportsToSol(lamports: number | BN): number {
  if (typeof lamports === 'number') {
    return Math.abs(lamports) / LAMPORTS_PER_SOL;
  }

  let signMultiplier = 1;
  if (lamports.isNeg()) {
    signMultiplier = -1;
  }

  const absLamports = lamports.abs();
  const lamportsString = absLamports.toString(10).padStart(10, '0');
  const splitIndex = lamportsString.length - 9;
  const solString = lamportsString.slice(0, splitIndex) + '.' + lamportsString.slice(splitIndex);
  return signMultiplier * parseFloat(solString);
}

/**
 * Convert the UI representation of a token amount (using the decimals field defined in its mint)
 * to the raw amount
 */
export function uiAmountToAmount(amount: number, decimals: number) {
  return getTokenMultiplierFromDecimals(decimals).toNumber() * amount;
}

/**
 * Convert a raw amount to its UI representation (using the decimals field defined in its mint)
 */
export function amountToUiAmount(amount: BN | number, decimals: number) {
  return divideBnToNumber(new BN(amount), getTokenMultiplierFromDecimals(decimals));
}

export function getTokenMultiplierFromDecimals(decimals: number): BN {
  return new BN(10).pow(new BN(decimals));
}

export const toBuffer = (arr: Buffer | Uint8Array | Array<number>): Buffer => {
  if (Buffer.isBuffer(arr)) {
    return arr;
  } else if (arr instanceof Uint8Array) {
    return Buffer.from(arr.buffer, arr.byteOffset, arr.byteLength);
  } else {
    return Buffer.from(arr);
  }
};

/**
 * Generates the withdraw authority program address for the stake pool
 */
export async function findWithdrawAuthorityProgramAddress(
  programId: PublicKey,
  stakePoolAddress: PublicKey,
) {
  const [publicKey] = await PublicKey.findProgramAddress(
    [stakePoolAddress.toBuffer(), Buffer.from('withdraw')],
    programId,
  );
  return publicKey;
}

/**
 * Generates the stake program address for a validator's vote account
 */
export async function findStakeProgramAddress(
  programId: PublicKey,
  voteAccountAddress: PublicKey,
  stakePoolAddress: PublicKey,
) {
  const [publicKey] = await PublicKey.findProgramAddress(
    [voteAccountAddress.toBuffer(), stakePoolAddress.toBuffer()],
    programId,
  );
  return publicKey;
}

/**
 * Generates the stake program address for a validator's vote account
 */
export async function findTransientStakeProgramAddress(
  programId: PublicKey,
  voteAccountAddress: PublicKey,
  stakePoolAddress: PublicKey,
  seed: BN,
) {
  const [publicKey] = await PublicKey.findProgramAddress(
    [
      TRANSIENT_STAKE_SEED_PREFIX,
      voteAccountAddress.toBuffer(),
      stakePoolAddress.toBuffer(),
      new Uint8Array(seed.toArray('le', 8)),
    ],
    programId,
  );
  return publicKey;
}

export async function getTokenMint(
  connection: Connection,
  tokenMintPubkey: PublicKey,
): Promise<MintInfo | undefined> {
  // @ts-ignore
  const token = new Token(connection, tokenMintPubkey, TOKEN_PROGRAM_ID, null);
  return token.getMintInfo();
}

/**
 * Retrieve the associated account or create one if not found.
 * This account may then be used as a `transfer()` or `approve()` destination
 */
export async function addAssociatedTokenAccount(
  connection: Connection,
  owner: PublicKey,
  mint: PublicKey,
  instructions: TransactionInstruction[],
) {
  const associatedAddress = await Token.getAssociatedTokenAddress(
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    mint,
    owner,
  );

  // This is the optimum logic, considering TX fee, client-side computation,
  // RPC roundtrips and guaranteed idempotent.
  // Sadly we can't do this atomically;
  try {
    const account = await connection.getAccountInfo(associatedAddress);
    if (!account) {
      // noinspection ExceptionCaughtLocallyJS
      throw new Error(FAILED_TO_FIND_ACCOUNT);
    }
  } catch (err: any) {
    // INVALID_ACCOUNT_OWNER can be possible if the associatedAddress has
    // already been received some lamports (= became system accounts).
    // Assuming program derived addressing is safe, this is the only case
    // for the INVALID_ACCOUNT_OWNER in this code-path
    if (err.message === FAILED_TO_FIND_ACCOUNT || err.message === INVALID_ACCOUNT_OWNER) {
      // as this isn't atomic, it's possible others can create associated
      // accounts meanwhile
      try {
        instructions.push(
          Token.createAssociatedTokenAccountInstruction(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            mint,
            associatedAddress,
            owner,
            owner,
          ),
        );
      } catch (err) {
        console.warn(err);
        // ignore all errors; for now there is no API compatible way to
        // selectively ignore the expected instruction error if the
        // associated account is existing already.
      }
    } else {
      throw err;
    }
    console.warn(err);
  }

  return associatedAddress;
}

export async function getTokenAccount(
  connection: Connection,
  tokenAccountAddress: PublicKey,
  expectedTokenMint: PublicKey,
): Promise<AccountInfo | void> {
  try {
    const account = await connection.getAccountInfo(tokenAccountAddress);
    if (!account) {
      // noinspection ExceptionCaughtLocallyJS
      throw new Error(`Invalid account ${tokenAccountAddress.toBase58()}`);
    }
    const tokenAccount = AccountLayout.decode(account.data) as AccountInfo;
    if (tokenAccount.mint?.toBase58() != expectedTokenMint.toBase58()) {
      // noinspection ExceptionCaughtLocallyJS
      throw new Error(
        `Invalid token mint for ${tokenAccountAddress}, expected mint is ${expectedTokenMint}`,
      );
    }
    return tokenAccount;
  } catch (error) {
    console.log(error);
  }
}

export async function getStakeAccountsByWithdrawAuthority(
  connection: Connection,
  withdrawAuthority: PublicKey,
) {
  return await connection.getParsedProgramAccounts(StakeProgram.programId, {
    filters: [
      // 44 is Withdrawer authority offset in stake account stake
      { memcmp: { offset: 44, bytes: withdrawAuthority.toBase58() } },
    ],
  });
}

export async function prepareWithdrawAccounts(
  connection: Connection,
  stakePool: StakePool,
  stakePoolAddress: PublicKey,
  amount: number,
): Promise<WithdrawAccount[]> {
  const validatorListAcc = await connection.getAccountInfo(stakePool.validatorList);
  const validatorList = ValidatorListLayout.decode(validatorListAcc!.data) as ValidatorList;

  if (!validatorList?.validators || validatorList?.validators.length == 0) {
    throw new Error('No accounts found');
  }

  let accounts = [] as Array<{
    type: 'preferred' | 'active' | 'transient' | 'reserve';
    voteAddress?: PublicKey | undefined;
    stakeAddress: PublicKey;
    lamports: number;
  }>;

  // Prepare accounts
  for (const validator of validatorList.validators) {

    if (validator.status !== 0) {
      // is not active status
      continue;
    }

    const stakeAccountAddress = await findStakeProgramAddress(
      StakePoolProgram.programId,
      validator.voteAccountAddress,
      stakePoolAddress,
    );

    if (!validator.activeStakeLamports.isZero()) {
      const isPreferred =
        stakePool.preferredWithdrawValidatorVoteAddress &&
        stakePool.preferredWithdrawValidatorVoteAddress!.toBase58() ==
        validator.voteAccountAddress.toBase58();
      accounts.push({
        type: isPreferred ? 'preferred' : 'active',
        voteAddress: validator.voteAccountAddress,
        stakeAddress: stakeAccountAddress,
        lamports: validator.activeStakeLamports.toNumber(),
      });
    }
    const transientStakeAccountAddress = await findTransientStakeProgramAddress(
      StakePoolProgram.programId,
      validator.voteAccountAddress,
      stakePoolAddress,
      validator.transientSeedSuffixStart!,
    );
    if (!validator.transientStakeLamports?.isZero()) {
      accounts.push({
        type: 'transient',
        voteAddress: validator.voteAccountAddress,
        stakeAddress: transientStakeAccountAddress,
        lamports: validator.transientStakeLamports!.toNumber(),
      });
    }
  }

  // Sort from highest to lowest balance
  accounts = accounts.sort((a, b) => b.lamports - a.lamports);

  if (stakePool.reserveStake) {
    const reserveStake = await connection.getAccountInfo(stakePool.reserveStake);
    if (reserveStake && reserveStake.lamports > 0) {
      console.log('Reserve Stake: ', reserveStake.lamports);
      accounts.push({
        type: 'reserve',
        stakeAddress: stakePool.reserveStake,
        lamports: reserveStake?.lamports,
      });
    }
  }

  // Prepare the list of accounts to withdraw from
  const withdrawFrom: WithdrawAccount[] = [];
  let remainingAmount = amount;

  for (const type of ['preferred', 'active', 'transient', 'reserve']) {
    const filteredAccounts = accounts.filter(a => a.type == type);
    // Max 5 accounts for type to prevent an error: "Transaction too large"
    // TODO: fix
    const maxAccountsByType = 5;
    let i = 0;
    for (const { stakeAddress, voteAddress, lamports } of filteredAccounts) {
      if (i >= maxAccountsByType) {
        break;
      }
      // TODO: check
      let availableForWithdrawal = Math.floor(calcPoolTokensForDeposit(stakePool, lamports));
      if (!stakePool.stakeWithdrawalFee.denominator.isZero()) {
        availableForWithdrawal = divideBnToNumber(
          new BN(availableForWithdrawal).mul(stakePool.stakeWithdrawalFee.denominator),
          stakePool.stakeWithdrawalFee.denominator.sub(stakePool.stakeWithdrawalFee.numerator),
        );
      }

      const poolAmount = Math.min(availableForWithdrawal, remainingAmount);
      if (poolAmount <= 0) {
        continue;
      }

      // Those accounts will be withdrawn completely with `claim` instruction
      withdrawFrom.push({ stakeAddress, voteAddress, poolAmount });
      remainingAmount -= poolAmount;
      if (remainingAmount == 0) {
        break;
      }
      i++;
    }
    if (remainingAmount == 0) {
      break;
    }
  }

  // Not enough stake to withdraw the specified amount
  if (remainingAmount > 0) {
    throw new Error(`No stake accounts found in this pool with enough balance to withdraw
        ${lamportsToSol(amount)} pool tokens.`);
  }

  return withdrawFrom;
}

/**
 * Calculate the pool tokens that should be minted for a deposit of `stakeLamports`
 */
export function calcPoolTokensForDeposit(stakePool: StakePool, stakeLamports: number): number {
  if (stakePool.poolTokenSupply.isZero() || stakePool.totalLamports.isZero()) {
    return stakeLamports;
  }
  return divideBnToNumber(
    new BN(stakeLamports).mul(stakePool.poolTokenSupply),
    stakePool.totalLamports,
  );
}

/**
 * Calculate lamports amount on withdrawal
 */
export function calcLamportsWithdrawAmount(stakePool: StakePool, poolTokens: number): number {
  // TODO: (checkedMul) overflow checking
  const numerator = new BN(poolTokens).mul(stakePool.totalLamports);
  const denominator = stakePool.poolTokenSupply;
  if (numerator.lt(denominator)) {
    return 0;
  }
  return divideBnToNumber(numerator, denominator);
}

export function divideBnToNumber(numerator: BN, denominator: BN): number {
  if (denominator.isZero()) {
    return 0;
  }
  const quotient = numerator.div(denominator).toNumber();
  const rem = numerator.umod(denominator);
  const gcd = rem.gcd(denominator);
  return quotient + rem.div(gcd).toNumber() / denominator.div(gcd).toNumber();
}

export function newStakeAccount(
  feePayer: PublicKey,
  instructions: TransactionInstruction[],
  lamports: number,
): Keypair {
  // Account for tokens not specified, creating one
  const stakeReceiverKeypair = Keypair.generate();
  console.log(`Creating account to receive stake ${stakeReceiverKeypair.publicKey}`);

  instructions.push(
    // Creating new account
    SystemProgram.createAccount({
      fromPubkey: feePayer,
      newAccountPubkey: stakeReceiverKeypair.publicKey,
      lamports,
      space: STAKE_STATE_LEN,
      programId: StakeProgram.programId,
    }),
  );

  return stakeReceiverKeypair;
}
