import {
  Keypair,
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import {
  approve,
  createMint,
  createAccount,
  createApproveInstruction,
  createInitializeAccountInstruction,
  getAccount,
  getMint,
  getMinimumBalanceForRentExemptAccount,
  mintTo,
  AccountLayout,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';

import {TokenSwap, CurveType, TOKEN_SWAP_PROGRAM_ID} from '../src';
import {newAccountWithLamports} from '../src/util/new-account-with-lamports';
import {sleep} from '../src/util/sleep';

describe('spl-token-swap instructions', () => {
  it('executes properly', async () => {
    // These test cases are designed to run sequentially and in the following order
    console.log('Run test: createTokenSwap (constant price)');
    const constantPrice = new Uint8Array(8);
    constantPrice[0] = 1;
    await createTokenSwap(CurveType.ConstantPrice, constantPrice);
    console.log(
      'Run test: createTokenSwap (constant product, used further in tests)',
    );
    await createTokenSwap(CurveType.ConstantProduct);
    console.log('Run test: deposit all token types');
    await depositAllTokenTypes();
    console.log('Run test: withdraw all token types');
    await withdrawAllTokenTypes();
    console.log('Run test: swap');
    await swap();
    console.log('Run test: create account, approve, swap all at once');
    await createAccountAndSwapAtomic();
    console.log('Run test: deposit one exact amount in');
    await depositSingleTokenTypeExactAmountIn();
    console.log('Run test: withdraw one exact amount out');
    await withdrawSingleTokenTypeExactAmountOut();
    console.log('Success\n');
  });
});

// The following globals are created by `createTokenSwap` and used by subsequent tests
// Token swap
let tokenSwap: TokenSwap;
// authority of the token and accounts
let authority: PublicKey;
// bump seed used to generate the authority public key
let bumpSeed: number;
// owner of the user accounts
let owner: Keypair;
// payer for transactions
let payer: Keypair;
// Token pool
let tokenPool: PublicKey;
let tokenAccountPool: PublicKey;
let feeAccount: PublicKey;
// Tokens swapped
let mintA: PublicKey;
const mintAProgramId: PublicKey = TOKEN_PROGRAM_ID;
let mintB: PublicKey;
const mintBProgramId: PublicKey = TOKEN_PROGRAM_ID;
let tokenAccountA: PublicKey;
let tokenAccountB: PublicKey;

// Hard-coded fee address, for testing production mode
const SWAP_PROGRAM_OWNER_FEE_ADDRESS =
  process.env.SWAP_PROGRAM_OWNER_FEE_ADDRESS;

// Pool fees
const TRADING_FEE_NUMERATOR = 25n;
const TRADING_FEE_DENOMINATOR = 10000n;
const OWNER_TRADING_FEE_NUMERATOR = 5n;
const OWNER_TRADING_FEE_DENOMINATOR = 10000n;
const OWNER_WITHDRAW_FEE_NUMERATOR = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 0n : 1n;
const OWNER_WITHDRAW_FEE_DENOMINATOR = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 0n : 6n;
const HOST_FEE_NUMERATOR = 20n;
const HOST_FEE_DENOMINATOR = 100n;

// Initial amount in each swap token
let currentSwapTokenA = 1000000n;
let currentSwapTokenB = 1000000n;
let currentFeeAmount = 0n;

// Swap instruction constants
// Because there is no withdraw fee in the production version, these numbers
// need to get slightly tweaked in the two cases.
const SWAP_AMOUNT_IN = 100000n;
const SWAP_AMOUNT_OUT = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 90661n : 90674n;
const SWAP_FEE = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 22727n : 22730n;
const HOST_SWAP_FEE = SWAP_PROGRAM_OWNER_FEE_ADDRESS
  ? (SWAP_FEE * HOST_FEE_NUMERATOR) / HOST_FEE_DENOMINATOR
  : 0n;
const OWNER_SWAP_FEE = SWAP_FEE - HOST_SWAP_FEE;

// Pool token amount minted on init
const DEFAULT_POOL_TOKEN_AMOUNT = 1000000000n;
// Pool token amount to withdraw / deposit
const POOL_TOKEN_AMOUNT = 10000000n;

function assert(condition: boolean, message?: string) {
  if (!condition) {
    console.log(Error().stack + ':token-test.js');
    throw message || 'Assertion failed';
  }
}

let connection: Connection;
async function getConnection(): Promise<Connection> {
  if (connection) return connection;

  const url = 'http://127.0.0.1:8899';
  connection = new Connection(url, 'processed');
  const version = await connection.getVersion();

  console.log('Connection to cluster established:', url, version);
  return connection;
}

export async function createTokenSwap(
  curveType: number,
  curveParameters?: Uint8Array,
): Promise<void> {
  const connection = await getConnection();
  payer = await newAccountWithLamports(connection, 1000000000);
  owner = await newAccountWithLamports(connection, 1000000000);
  const tokenSwapAccount = Keypair.generate();

  [authority, bumpSeed] = await PublicKey.findProgramAddress(
    [tokenSwapAccount.publicKey.toBuffer()],
    TOKEN_SWAP_PROGRAM_ID,
  );

  console.log('creating pool mint');
  tokenPool = await createMint(
    connection,
    payer,
    authority,
    null,
    2,
    Keypair.generate(),
    undefined,
    TOKEN_PROGRAM_ID,
  );

  console.log('creating pool account');
  tokenAccountPool = await createAccount(
    connection,
    payer,
    tokenPool,
    owner.publicKey,
    Keypair.generate(),
  );
  const ownerKey = SWAP_PROGRAM_OWNER_FEE_ADDRESS || owner.publicKey.toString();
  feeAccount = await createAccount(
    connection,
    payer,
    tokenPool,
    new PublicKey(ownerKey),
    Keypair.generate(),
  );

  console.log('creating token A');
  mintA = await createMint(
    connection,
    payer,
    owner.publicKey,
    null,
    2,
    Keypair.generate(),
    undefined,
    mintAProgramId,
  );

  console.log('creating token A account');
  tokenAccountA = await createAccount(
    connection,
    payer,
    mintA,
    authority,
    Keypair.generate(),
  );
  console.log('minting token A to swap');
  await mintTo(
    connection,
    payer,
    mintA,
    tokenAccountA,
    owner,
    currentSwapTokenA,
  );

  console.log('creating token B');
  mintB = await createMint(
    connection,
    payer,
    owner.publicKey,
    null,
    2,
    Keypair.generate(),
    undefined,
    mintBProgramId,
  );

  console.log('creating token B account');
  tokenAccountB = await createAccount(
    connection,
    payer,
    mintB,
    authority,
    Keypair.generate(),
  );
  console.log('minting token B to swap');
  await mintTo(
    connection,
    payer,
    mintB,
    tokenAccountB,
    owner,
    currentSwapTokenB,
  );

  console.log('creating token swap');
  const swapPayer = await newAccountWithLamports(connection, 10000000000);
  tokenSwap = await TokenSwap.createTokenSwap(
    connection,
    swapPayer,
    tokenSwapAccount,
    authority,
    tokenAccountA,
    tokenAccountB,
    tokenPool,
    mintA,
    mintB,
    feeAccount,
    tokenAccountPool,
    TOKEN_SWAP_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    TRADING_FEE_NUMERATOR,
    TRADING_FEE_DENOMINATOR,
    OWNER_TRADING_FEE_NUMERATOR,
    OWNER_TRADING_FEE_DENOMINATOR,
    OWNER_WITHDRAW_FEE_NUMERATOR,
    OWNER_WITHDRAW_FEE_DENOMINATOR,
    HOST_FEE_NUMERATOR,
    HOST_FEE_DENOMINATOR,
    curveType,
    curveParameters,
  );

  console.log('loading token swap');
  const fetchedTokenSwap = await TokenSwap.loadTokenSwap(
    connection,
    tokenSwapAccount.publicKey,
    TOKEN_SWAP_PROGRAM_ID,
    swapPayer,
  );

  assert(fetchedTokenSwap.poolTokenProgramId.equals(TOKEN_PROGRAM_ID));
  assert(fetchedTokenSwap.tokenAccountA.equals(tokenAccountA));
  assert(fetchedTokenSwap.tokenAccountB.equals(tokenAccountB));
  assert(fetchedTokenSwap.mintA.equals(mintA));
  assert(fetchedTokenSwap.mintB.equals(mintB));
  assert(fetchedTokenSwap.poolToken.equals(tokenPool));
  assert(fetchedTokenSwap.feeAccount.equals(feeAccount));
  assert(TRADING_FEE_NUMERATOR == fetchedTokenSwap.tradeFeeNumerator);
  assert(TRADING_FEE_DENOMINATOR == fetchedTokenSwap.tradeFeeDenominator);
  assert(
    OWNER_TRADING_FEE_NUMERATOR == fetchedTokenSwap.ownerTradeFeeNumerator,
  );
  assert(
    OWNER_TRADING_FEE_DENOMINATOR == fetchedTokenSwap.ownerTradeFeeDenominator,
  );
  assert(
    OWNER_WITHDRAW_FEE_NUMERATOR == fetchedTokenSwap.ownerWithdrawFeeNumerator,
  );
  assert(
    OWNER_WITHDRAW_FEE_DENOMINATOR ==
      fetchedTokenSwap.ownerWithdrawFeeDenominator,
  );
  assert(HOST_FEE_NUMERATOR == fetchedTokenSwap.hostFeeNumerator);
  assert(HOST_FEE_DENOMINATOR == fetchedTokenSwap.hostFeeDenominator);
  assert(curveType == fetchedTokenSwap.curveType);
}

export async function depositAllTokenTypes(): Promise<void> {
  const poolMintInfo = await getMint(connection, tokenPool);
  const supply = poolMintInfo.supply;
  const swapTokenA = await getAccount(connection, tokenAccountA);
  const tokenA = (swapTokenA.amount * BigInt(POOL_TOKEN_AMOUNT)) / supply;
  const swapTokenB = await getAccount(connection, tokenAccountB);
  const tokenB = (swapTokenB.amount * BigInt(POOL_TOKEN_AMOUNT)) / supply;

  const userTransferAuthority = Keypair.generate();
  console.log('Creating depositor token a account');
  const userAccountA = await createAccount(
    connection,
    payer,
    mintA,
    owner.publicKey,
    Keypair.generate(),
  );
  await mintTo(connection, payer, mintA, userAccountA, owner, tokenA);
  await approve(
    connection,
    payer,
    userAccountA,
    userTransferAuthority.publicKey,
    owner,
    tokenA,
  );
  console.log('Creating depositor token b account');
  const userAccountB = await createAccount(
    connection,
    payer,
    mintB,
    owner.publicKey,
    Keypair.generate(),
  );
  await mintTo(connection, payer, mintB, userAccountB, owner, tokenB);
  await approve(
    connection,
    payer,
    userAccountB,
    userTransferAuthority.publicKey,
    owner,
    tokenB,
  );
  console.log('Creating depositor pool token account');
  const newAccountPool = await createAccount(
    connection,
    payer,
    tokenPool,
    owner.publicKey,
    Keypair.generate(),
  );

  const confirmOptions = {
    skipPreflight: true,
  };

  console.log('Depositing into swap');
  await tokenSwap.depositAllTokenTypes(
    userAccountA,
    userAccountB,
    newAccountPool,
    TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    userTransferAuthority,
    POOL_TOKEN_AMOUNT,
    tokenA,
    tokenB,
    confirmOptions,
  );

  let info;
  info = await getAccount(connection, userAccountA);
  assert(info.amount == 0n);
  info = await getAccount(connection, userAccountB);
  assert(info.amount == 0n);
  info = await getAccount(connection, tokenAccountA);
  assert(info.amount == currentSwapTokenA + tokenA);
  currentSwapTokenA += tokenA;
  info = await getAccount(connection, tokenAccountB);
  assert(info.amount == currentSwapTokenB + tokenB);
  currentSwapTokenB += tokenB;
  info = await getAccount(connection, newAccountPool);
  assert(info.amount == POOL_TOKEN_AMOUNT);
}

export async function withdrawAllTokenTypes(): Promise<void> {
  const poolMintInfo = await getMint(connection, tokenPool);
  const supply = poolMintInfo.supply;
  let swapTokenA = await getAccount(connection, tokenAccountA);
  let swapTokenB = await getAccount(connection, tokenAccountB);
  let feeAmount = 0n;
  if (OWNER_WITHDRAW_FEE_NUMERATOR !== 0n) {
    feeAmount =
      (POOL_TOKEN_AMOUNT * OWNER_WITHDRAW_FEE_NUMERATOR) /
      OWNER_WITHDRAW_FEE_DENOMINATOR;
  }
  const poolTokenAmount = POOL_TOKEN_AMOUNT - feeAmount;
  const tokenA = (swapTokenA.amount * BigInt(poolTokenAmount)) / supply;
  const tokenB = (swapTokenB.amount * BigInt(poolTokenAmount)) / supply;

  console.log('Creating withdraw token A account');
  const userAccountA = await createAccount(
    connection,
    payer,
    mintA,
    owner.publicKey,
    Keypair.generate(),
  );
  console.log('Creating withdraw token B account');
  const userAccountB = await createAccount(
    connection,
    payer,
    mintB,
    owner.publicKey,
    Keypair.generate(),
  );

  const userTransferAuthority = Keypair.generate();
  console.log('Approving withdrawal from pool account');
  await approve(
    connection,
    payer,
    tokenAccountPool,
    userTransferAuthority.publicKey,
    owner,
    POOL_TOKEN_AMOUNT,
  );

  const confirmOptions = {
    skipPreflight: true,
  };

  console.log('Withdrawing pool tokens for A and B tokens');
  await tokenSwap.withdrawAllTokenTypes(
    userAccountA,
    userAccountB,
    tokenAccountPool,
    TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    userTransferAuthority,
    POOL_TOKEN_AMOUNT,
    tokenA,
    tokenB,
    confirmOptions,
  );

  //const poolMintInfo = await tokenPool.getMintInfo();
  swapTokenA = await getAccount(connection, tokenAccountA);
  swapTokenB = await getAccount(connection, tokenAccountB);

  let info = await getAccount(connection, tokenAccountPool);
  assert(info.amount == DEFAULT_POOL_TOKEN_AMOUNT - POOL_TOKEN_AMOUNT);
  assert(swapTokenA.amount == currentSwapTokenA - tokenA);
  currentSwapTokenA -= tokenA;
  assert(swapTokenB.amount == currentSwapTokenB - tokenB);
  currentSwapTokenB -= tokenB;
  info = await getAccount(connection, userAccountA);
  assert(info.amount == tokenA);
  info = await getAccount(connection, userAccountB);
  assert(info.amount == tokenB);
  info = await getAccount(connection, feeAccount);
  assert(info.amount == feeAmount);
  currentFeeAmount = feeAmount;
}

export async function createAccountAndSwapAtomic(): Promise<void> {
  console.log('Creating swap token a account');
  const userAccountA = await createAccount(
    connection,
    payer,
    mintA,
    owner.publicKey,
    Keypair.generate(),
  );
  await mintTo(connection, payer, mintA, userAccountA, owner, SWAP_AMOUNT_IN);

  // @ts-ignore
  const balanceNeeded = await getMinimumBalanceForRentExemptAccount(connection);
  const newAccount = Keypair.generate();
  const transaction = new Transaction();
  transaction.add(
    SystemProgram.createAccount({
      fromPubkey: owner.publicKey,
      newAccountPubkey: newAccount.publicKey,
      lamports: balanceNeeded,
      space: AccountLayout.span,
      programId: mintBProgramId,
    }),
  );

  transaction.add(
    createInitializeAccountInstruction(
      newAccount.publicKey,
      mintB,
      owner.publicKey,
      mintBProgramId,
    ),
  );

  const userTransferAuthority = Keypair.generate();
  transaction.add(
    createApproveInstruction(
      userAccountA,
      userTransferAuthority.publicKey,
      owner.publicKey,
      SWAP_AMOUNT_IN,
      [],
      mintAProgramId,
    ),
  );

  transaction.add(
    TokenSwap.swapInstruction(
      tokenSwap.tokenSwap,
      tokenSwap.authority,
      userTransferAuthority.publicKey,
      userAccountA,
      tokenSwap.tokenAccountA,
      tokenSwap.tokenAccountB,
      newAccount.publicKey,
      tokenSwap.poolToken,
      tokenSwap.feeAccount,
      null,
      tokenSwap.mintA,
      tokenSwap.mintB,
      tokenSwap.swapProgramId,
      TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      tokenSwap.poolTokenProgramId,
      SWAP_AMOUNT_IN,
      0n,
    ),
  );

  const confirmOptions = {
    skipPreflight: true,
  };

  // Send the instructions
  console.log('sending big instruction');
  await sendAndConfirmTransaction(
    connection,
    transaction,
    [payer, owner, newAccount, userTransferAuthority],
    confirmOptions,
  );

  let info;
  info = await getAccount(connection, tokenAccountA);
  currentSwapTokenA = info.amount;
  info = await getAccount(connection, tokenAccountB);
  currentSwapTokenB = info.amount;
}

export async function swap(): Promise<void> {
  console.log('Creating swap token a account');
  const userAccountA = await createAccount(
    connection,
    payer,
    mintA,
    owner.publicKey,
    Keypair.generate(),
  );
  await mintTo(connection, payer, mintA, userAccountA, owner, SWAP_AMOUNT_IN);
  const userTransferAuthority = Keypair.generate();
  await approve(
    connection,
    payer,
    userAccountA,
    userTransferAuthority.publicKey,
    owner,
    SWAP_AMOUNT_IN,
  );
  console.log('Creating swap token b account');
  const userAccountB = await createAccount(
    connection,
    payer,
    mintB,
    owner.publicKey,
    Keypair.generate(),
  );
  const poolAccount = SWAP_PROGRAM_OWNER_FEE_ADDRESS
    ? await createAccount(
        connection,
        payer,
        tokenPool,
        owner.publicKey,
        Keypair.generate(),
      )
    : null;

  const confirmOptions = {
    skipPreflight: true,
  };

  console.log('Swapping');
  await tokenSwap.swap(
    userAccountA,
    tokenAccountA,
    tokenAccountB,
    userAccountB,
    tokenSwap.mintA,
    tokenSwap.mintB,
    TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    poolAccount,
    userTransferAuthority,
    SWAP_AMOUNT_IN,
    SWAP_AMOUNT_OUT,
    confirmOptions,
  );

  await sleep(500);

  let info;
  info = await getAccount(connection, userAccountA);
  assert(info.amount == 0n);

  info = await getAccount(connection, userAccountB);
  assert(info.amount == SWAP_AMOUNT_OUT);

  info = await getAccount(connection, tokenAccountA);
  assert(info.amount == currentSwapTokenA + SWAP_AMOUNT_IN);
  currentSwapTokenA += SWAP_AMOUNT_IN;

  info = await getAccount(connection, tokenAccountB);
  assert(info.amount == currentSwapTokenB - SWAP_AMOUNT_OUT);
  currentSwapTokenB -= SWAP_AMOUNT_OUT;

  info = await getAccount(connection, tokenAccountPool);
  assert(info.amount == DEFAULT_POOL_TOKEN_AMOUNT - POOL_TOKEN_AMOUNT);

  info = await getAccount(connection, feeAccount);
  assert(info.amount == currentFeeAmount + OWNER_SWAP_FEE);

  if (poolAccount != null) {
    info = await getAccount(connection, poolAccount);
    assert(info.amount == HOST_SWAP_FEE);
  }
}

function tradingTokensToPoolTokens(
  sourceAmount: bigint,
  swapSourceAmount: bigint,
  poolAmount: bigint,
): bigint {
  const tradingFee =
    (sourceAmount / 2n) * (TRADING_FEE_NUMERATOR / TRADING_FEE_DENOMINATOR);
  const ownerTradingFee =
    (sourceAmount / 2n) *
    (OWNER_TRADING_FEE_NUMERATOR / OWNER_TRADING_FEE_DENOMINATOR);
  const sourceAmountPostFee = sourceAmount - tradingFee - ownerTradingFee;
  const root = Math.sqrt(
    Number(sourceAmountPostFee) / Number(swapSourceAmount) + 1,
  );
  return BigInt(Math.floor(Number(poolAmount) * (root - 1)));
}

export async function depositSingleTokenTypeExactAmountIn(): Promise<void> {
  // Pool token amount to deposit on one side
  const depositAmount = 10000n;

  const poolMintInfo = await getMint(connection, tokenPool);
  const supply = poolMintInfo.supply;
  const swapTokenA = await getAccount(connection, tokenAccountA);
  //const poolTokenA = tradingTokensToPoolTokens(
  //  depositAmount,
  //  swapTokenA.amount,
  //  supply,
  //);
  const poolTokenA = 0n; // maybe do this better eventually
  const swapTokenB = await getAccount(connection, tokenAccountB);
  //const poolTokenB = tradingTokensToPoolTokens(
  //  depositAmount,
  //  swapTokenB.amount,
  //  supply,
  //;
  const poolTokenB = 0n; // maybe do this better eventually

  const userTransferAuthority = Keypair.generate();
  console.log('Creating depositor token a account');
  const userAccountA = await createAccount(
    connection,
    payer,
    mintA,
    owner.publicKey,
    Keypair.generate(),
  );
  await mintTo(connection, payer, mintA, userAccountA, owner, depositAmount);
  await approve(
    connection,
    payer,
    userAccountA,
    userTransferAuthority.publicKey,
    owner,
    depositAmount,
  );
  console.log('Creating depositor token b account');
  const userAccountB = await createAccount(
    connection,
    payer,
    mintB,
    owner.publicKey,
    Keypair.generate(),
  );
  await mintTo(connection, payer, mintB, userAccountB, owner, depositAmount);
  await approve(
    connection,
    payer,
    userAccountB,
    userTransferAuthority.publicKey,
    owner,
    depositAmount,
  );
  console.log('Creating depositor pool token account');
  const newAccountPool = await createAccount(
    connection,
    payer,
    tokenPool,
    owner.publicKey,
    Keypair.generate(),
  );

  const confirmOptions = {
    skipPreflight: true,
  };

  console.log('Depositing token A into swap');
  await tokenSwap.depositSingleTokenTypeExactAmountIn(
    userAccountA,
    newAccountPool,
    tokenSwap.mintA,
    TOKEN_PROGRAM_ID,
    userTransferAuthority,
    depositAmount,
    poolTokenA,
    confirmOptions,
  );

  let info;
  info = await getAccount(connection, userAccountA);
  assert(info.amount == 0n);
  info = await getAccount(connection, tokenAccountA);
  assert(info.amount == currentSwapTokenA + depositAmount);
  currentSwapTokenA += depositAmount;

  console.log('Depositing token B into swap');
  await tokenSwap.depositSingleTokenTypeExactAmountIn(
    userAccountB,
    newAccountPool,
    tokenSwap.mintB,
    TOKEN_PROGRAM_ID,
    userTransferAuthority,
    depositAmount,
    poolTokenB,
    confirmOptions,
  );

  info = await getAccount(connection, userAccountB);
  assert(info.amount == 0n);
  info = await getAccount(connection, tokenAccountB);
  assert(info.amount == currentSwapTokenB + depositAmount);
  currentSwapTokenB += depositAmount;
  info = await getAccount(connection, newAccountPool);
  assert(info.amount >= poolTokenA + poolTokenB);
}

export async function withdrawSingleTokenTypeExactAmountOut(): Promise<void> {
  // Pool token amount to withdraw on one side
  const withdrawAmount = 50000n;

  const poolMintInfo = await getMint(connection, tokenPool);
  const supply = poolMintInfo.supply;

  const swapTokenA = await getAccount(connection, tokenAccountA);
  const swapTokenAPost = swapTokenA.amount - withdrawAmount;
  //const poolTokenA = tradingTokensToPoolTokens(
  //  withdrawAmount,
  //  swapTokenAPost,
  //  supply,
  //);
  //if (OWNER_WITHDRAW_FEE_NUMERATOR !== 0n) {
  //  adjustedPoolTokenA *=
  //    1n + OWNER_WITHDRAW_FEE_NUMERATOR / OWNER_WITHDRAW_FEE_DENOMINATOR;
  //}
  const adjustedPoolTokenA = 1_000_000_000_000n; // maybe do this better

  const swapTokenB = await getAccount(connection, tokenAccountB);
  const swapTokenBPost = swapTokenB.amount - withdrawAmount;
  //const poolTokenB = tradingTokensToPoolTokens(
  //  withdrawAmount,
  //  swapTokenBPost,
  //  supply,
  //);
  //if (OWNER_WITHDRAW_FEE_NUMERATOR !== 0n) {
  //  adjustedPoolTokenB *=
  //    1n + OWNER_WITHDRAW_FEE_NUMERATOR / OWNER_WITHDRAW_FEE_DENOMINATOR;
  //}
  const adjustedPoolTokenB = 1_000_000_000_000n; // maybe do this better

  const userTransferAuthority = Keypair.generate();
  console.log('Creating withdraw token a account');
  const userAccountA = await createAccount(
    connection,
    payer,
    mintA,
    owner.publicKey,
    Keypair.generate(),
  );
  console.log('Creating withdraw token b account');
  const userAccountB = await createAccount(
    connection,
    payer,
    mintB,
    owner.publicKey,
    Keypair.generate(),
  );
  console.log('Creating withdraw pool token account');
  const poolAccount = await getAccount(connection, tokenAccountPool);
  const poolTokenAmount = poolAccount.amount;
  await approve(
    connection,
    payer,
    tokenAccountPool,
    userTransferAuthority.publicKey,
    owner,
    adjustedPoolTokenA + adjustedPoolTokenB,
  );

  const confirmOptions = {
    skipPreflight: true,
  };

  console.log('Withdrawing token A only');
  await tokenSwap.withdrawSingleTokenTypeExactAmountOut(
    userAccountA,
    tokenAccountPool,
    tokenSwap.mintA,
    TOKEN_PROGRAM_ID,
    userTransferAuthority,
    withdrawAmount,
    adjustedPoolTokenA,
    confirmOptions,
  );

  let info;
  info = await getAccount(connection, userAccountA);
  assert(info.amount == withdrawAmount);
  info = await getAccount(connection, tokenAccountA);
  assert(info.amount == currentSwapTokenA - withdrawAmount);
  currentSwapTokenA += withdrawAmount;
  info = await getAccount(connection, tokenAccountPool);
  assert(info.amount >= poolTokenAmount - adjustedPoolTokenA);

  console.log('Withdrawing token B only');
  await tokenSwap.withdrawSingleTokenTypeExactAmountOut(
    userAccountB,
    tokenAccountPool,
    tokenSwap.mintB,
    TOKEN_PROGRAM_ID,
    userTransferAuthority,
    withdrawAmount,
    adjustedPoolTokenB,
    confirmOptions,
  );

  info = await getAccount(connection, userAccountB);
  assert(info.amount == withdrawAmount);
  info = await getAccount(connection, tokenAccountB);
  assert(info.amount == currentSwapTokenB - withdrawAmount);
  currentSwapTokenB += withdrawAmount;
  info = await getAccount(connection, tokenAccountPool);
  assert(
    info.amount >= poolTokenAmount - adjustedPoolTokenA - adjustedPoolTokenB,
  );
}
