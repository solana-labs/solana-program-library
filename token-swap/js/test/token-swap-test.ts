import {
  Keypair,
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import {
  AccountLayout,
  Token,
  TOKEN_PROGRAM_ID,
  NATIVE_MINT,
} from '@solana/spl-token';

import {
  TokenSwap,
  CurveType,
  STEP_SWAP_PROGRAM_ID,
  POOL_REGISTRY_SEED,
} from '../src';
import {sendAndConfirmTransaction} from '../src/util/send-and-confirm-transaction';
import {newAccountWithLamports} from '../src/util/new-account-with-lamports';
import {sleep} from '../src/util/sleep';

// The following globals are created by `createTokenSwap` and used by subsequent tests
// Token swap
let tokenSwap: TokenSwap;
// authority of the token and accounts
let authority: PublicKey;
// nonce used to generate the authority public key
let nonce: number;
// owner of the user accounts
let owner: Keypair;
// Token pool
let tokenPool: Token;
let tokenAccountPool: PublicKey;
let feeAccount: PublicKey;
// Tokens swapped
let mintA: Token;
let mintB: Token;
let tokenAccountA: PublicKey;
let tokenAccountB: PublicKey;

//for routed swaps, a second swap and a third mint
let tokenSwap2: TokenSwap;
let authority2: PublicKey;
let nonce2: number;
let mintC: Token;
let tokenAccountB2: PublicKey;
let tokenAccountC: PublicKey;

let tokenPool2: Token;
let tokenAccountPool2: PublicKey;
let feeAccount2: PublicKey;

//for wsol swaps, a third swap and a native mint
let tokenSwap3: TokenSwap;
let authority3: PublicKey;
let nonce3: number;
let mintNativePubkey: PublicKey = NATIVE_MINT;
let tokenAccountANativeSwap: PublicKey;
let tokenAccountBNativeSwap: PublicKey;

let tokenPool3: Token;
let tokenAccountPool3: PublicKey;
let feeAccount3: PublicKey;

//payer
let payer: Keypair;

// Hard-coded fee address, for testing production mode
const SWAP_PROGRAM_OWNER_FEE_ADDRESS =
  process.env.SWAP_PROGRAM_OWNER_FEE_ADDRESS;

// Pool fees
const TRADING_FEE_NUMERATOR = 25;
const TRADING_FEE_DENOMINATOR = 10000;
const OWNER_TRADING_FEE_NUMERATOR = 5;
const OWNER_TRADING_FEE_DENOMINATOR = 10000;
const OWNER_WITHDRAW_FEE_NUMERATOR = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 0 : 1;
const OWNER_WITHDRAW_FEE_DENOMINATOR = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 0 : 6;

// curve type used to calculate swaps and deposits
const CURVE_TYPE = CurveType.ConstantProduct;

// Initial amount in each swap token
let currentSwapTokenA = 1000000;
let currentSwapTokenB = 1000000;
let currentSwapTokenB2 = 1000000;
let currentSwapTokenC = 1000000;
let currentSwapTokenANative = 1000000;
let currentSwapTokenBNative = 1000000;
let currentFeeAmount = 0;

// Swap instruction constants
// Because there is no withdraw fee in the production version, these numbers
// need to get slightly tweaked in the two cases.
const SWAP_AMOUNT_IN = 100000;
const SWAP_AMOUNT_OUT = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 90661 : 90674;

const SWAP_FEE = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 22273 : 22277;
const OWNER_SWAP_FEE = SWAP_FEE;

const ROUTED_SWAP_AMOUNT_OUT1 = SWAP_PROGRAM_OWNER_FEE_ADDRESS
  ? /*?*/ 0
  : 90677;
const ROUTED_SWAP_AMOUNT_OUT2 = SWAP_PROGRAM_OWNER_FEE_ADDRESS
  ? /*?*/ 0
  : 82925;

const OWNER_ROUTED_SWAP_FEE1 = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? /*?*/ 0 : 13182;
const OWNER_ROUTED_SWAP_FEE2 = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? /*?*/ 0 : 11920;

// Pool token amount minted on init
const DEFAULT_POOL_TOKEN_AMOUNT = 1000000000;
// Pool token amount to withdraw / deposit
const POOL_TOKEN_AMOUNT = 10000000;

function assert(condition: boolean, message?: string) {
  if (!condition) {
    console.log(Error().stack + ':token-test.js');
    throw message || 'Assertion failed';
  }
}

let connection: Connection;
async function getConnection(): Promise<Connection> {
  if (connection) return connection;

  const url = 'http://localhost:8899';
  connection = new Connection(url, 'recent');
  const version = await connection.getVersion();

  console.log('Connection to cluster established:', url, version);
  return connection;
}

export async function initializePoolRegistry(): Promise<void> {
  //reset these - to allow for rerun
  currentSwapTokenA = 1000000;
  currentSwapTokenB = 1000000;
  currentSwapTokenB2 = 1000000;
  currentSwapTokenC = 1000000;
  currentSwapTokenANative = 1000000;
  currentSwapTokenBNative = 1000000;
  currentFeeAmount = 0;

  const connection = await getConnection();
  payer = await newAccountWithLamports(connection, 172981613440);
  const transaction = await TokenSwap.initializePoolRegistry(
    connection,
    payer.publicKey,
    STEP_SWAP_PROGRAM_ID,
  );

  await sendAndConfirmTransaction(
    'initialize pool registry',
    connection,
    transaction,
    payer,
  );
}

export async function createTokenSwap(): Promise<void> {
  const connection = await getConnection();
  owner = await newAccountWithLamports(connection, 1000000000);

  console.log('creating token A');
  mintA = await Token.createMint(
    connection,
    payer,
    owner.publicKey,
    null,
    2,
    TOKEN_PROGRAM_ID,
  );

  console.log('creating token B');
  mintB = await Token.createMint(
    connection,
    payer,
    owner.publicKey,
    null,
    2,
    TOKEN_PROGRAM_ID,
  );

  const seedKeyVec = [mintA.publicKey.toBuffer(), mintB.publicKey.toBuffer()];
  seedKeyVec.sort(Buffer.compare);

  const curveBuffer = Buffer.alloc(1);
  curveBuffer.writeUInt8(CURVE_TYPE, 0);
  let [tokenSwapKey, poolNonce] = await PublicKey.findProgramAddress(
    [seedKeyVec[0], seedKeyVec[1], curveBuffer],
    STEP_SWAP_PROGRAM_ID,
  );

  [authority, nonce] = await PublicKey.findProgramAddress(
    [tokenSwapKey.toBuffer()],
    STEP_SWAP_PROGRAM_ID,
  );

  console.log('creating pool mint');
  tokenPool = await Token.createMint(
    connection,
    payer,
    authority,
    null,
    2,
    TOKEN_PROGRAM_ID,
  );

  console.log('creating pool account');
  tokenAccountPool = await tokenPool.createAccount(owner.publicKey);
  const ownerKey = SWAP_PROGRAM_OWNER_FEE_ADDRESS || owner.publicKey.toString();
  feeAccount = await tokenPool.createAccount(new PublicKey(ownerKey));

  console.log('creating token A account');
  tokenAccountA = await mintA.createAccount(authority);
  console.log('minting token A to swap');
  await mintA.mintTo(tokenAccountA, owner, [], currentSwapTokenA);

  console.log('creating token B account');
  tokenAccountB = await mintB.createAccount(authority);
  console.log('minting token B to swap');
  await mintB.mintTo(tokenAccountB, owner, [], currentSwapTokenB);

  const poolRegistryKey = await PublicKey.createWithSeed(
    payer.publicKey,
    POOL_REGISTRY_SEED,
    STEP_SWAP_PROGRAM_ID,
  );

  console.log('creating token swap');
  tokenSwap = await TokenSwap.createTokenSwap(
    connection,
    owner,
    tokenSwapKey,
    authority,
    tokenAccountA,
    tokenAccountB,
    tokenPool.publicKey,
    mintA.publicKey,
    mintB.publicKey,
    feeAccount,
    tokenAccountPool,
    STEP_SWAP_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    nonce,
    TRADING_FEE_NUMERATOR,
    TRADING_FEE_DENOMINATOR,
    OWNER_TRADING_FEE_NUMERATOR,
    OWNER_TRADING_FEE_DENOMINATOR,
    OWNER_WITHDRAW_FEE_NUMERATOR,
    OWNER_WITHDRAW_FEE_DENOMINATOR,
    CURVE_TYPE,
    poolRegistryKey,
    poolNonce,
  );

  console.log('loading token swap');
  const fetchedTokenSwap = await TokenSwap.loadTokenSwap(
    connection,
    tokenSwapKey,
    STEP_SWAP_PROGRAM_ID,
    payer.publicKey,
    payer,
  );

  const poolRegistry = await TokenSwap.loadPoolRegistry(
    connection,
    payer.publicKey,
    STEP_SWAP_PROGRAM_ID,
  );

  if (!poolRegistry) {
    assert(poolRegistry !== undefined);
    return;
  }
  assert(poolRegistry.isInitialized);
  assert(poolRegistry.registrySize == 1);
  assert(
    poolRegistry.accounts[poolRegistry.registrySize - 1].equals(tokenSwapKey),
  );

  assert(fetchedTokenSwap.tokenProgramId.equals(TOKEN_PROGRAM_ID));
  assert(fetchedTokenSwap.tokenAccountA.equals(tokenAccountA));
  assert(fetchedTokenSwap.tokenAccountB.equals(tokenAccountB));
  assert(fetchedTokenSwap.mintA.equals(mintA.publicKey));
  assert(fetchedTokenSwap.mintB.equals(mintB.publicKey));
  assert(fetchedTokenSwap.poolToken.equals(tokenPool.publicKey));
  assert(fetchedTokenSwap.feeAccount.equals(feeAccount));
  assert(
    TRADING_FEE_NUMERATOR == fetchedTokenSwap.tradeFeeNumerator.toNumber(),
  );
  assert(
    TRADING_FEE_DENOMINATOR == fetchedTokenSwap.tradeFeeDenominator.toNumber(),
  );
  assert(
    OWNER_TRADING_FEE_NUMERATOR ==
      fetchedTokenSwap.ownerTradeFeeNumerator.toNumber(),
  );
  assert(
    OWNER_TRADING_FEE_DENOMINATOR ==
      fetchedTokenSwap.ownerTradeFeeDenominator.toNumber(),
  );
  assert(
    OWNER_WITHDRAW_FEE_NUMERATOR ==
      fetchedTokenSwap.ownerWithdrawFeeNumerator.toNumber(),
  );
  assert(
    OWNER_WITHDRAW_FEE_DENOMINATOR ==
      fetchedTokenSwap.ownerWithdrawFeeDenominator.toNumber(),
  );
  assert(CURVE_TYPE == fetchedTokenSwap.curveType);
  assert(poolNonce == fetchedTokenSwap.poolNonce);

  let success = await tryCreateTokenSwap(CURVE_TYPE);
  assert(!success);
  success = await tryCreateTokenSwap(CurveType.Stable);
  assert(success);
}

export async function tryCreateTokenSwap(curveType: number): Promise<boolean> {
  console.log('attempting to create duplicate pool with curve: ', curveType);
  const connection = await getConnection();

  const seedKeyVec = [mintA.publicKey.toBuffer(), mintB.publicKey.toBuffer()];
  seedKeyVec.sort();

  const curveBuffer = Buffer.alloc(1);
  curveBuffer.writeUInt8(curveType, 0);
  let [tokenSwapKey, poolNonce] = await PublicKey.findProgramAddress(
    [seedKeyVec[0], seedKeyVec[1], curveBuffer],
    STEP_SWAP_PROGRAM_ID,
  );

  let [authority, nonce] = await PublicKey.findProgramAddress(
    [tokenSwapKey.toBuffer()],
    STEP_SWAP_PROGRAM_ID,
  );

  console.log('creating pool mint');
  const tokenPool = await Token.createMint(
    connection,
    payer,
    authority,
    null,
    2,
    TOKEN_PROGRAM_ID,
  );

  console.log('creating pool account');
  const tokenAccountPool = await tokenPool.createAccount(owner.publicKey);
  const ownerKey = SWAP_PROGRAM_OWNER_FEE_ADDRESS || owner.publicKey.toString();
  const feeAccount = await tokenPool.createAccount(new PublicKey(ownerKey));

  console.log('creating token A account');
  const tokenAccountA = await mintA.createAccount(authority);
  console.log('minting token A to swap');
  await mintA.mintTo(tokenAccountA, owner, [], currentSwapTokenA);

  console.log('creating token B account');
  const tokenAccountB = await mintB.createAccount(authority);
  console.log('minting token B to swap');
  await mintB.mintTo(tokenAccountB, owner, [], currentSwapTokenB);

  const poolRegistryKey = await PublicKey.createWithSeed(
    payer.publicKey,
    POOL_REGISTRY_SEED,
    STEP_SWAP_PROGRAM_ID,
  );

  console.log('creating token swap');
  try {
    await TokenSwap.createTokenSwap(
      connection,
      payer,
      tokenSwapKey,
      authority,
      tokenAccountA,
      tokenAccountB,
      tokenPool.publicKey,
      mintA.publicKey,
      mintB.publicKey,
      feeAccount,
      tokenAccountPool,
      STEP_SWAP_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      nonce,
      TRADING_FEE_NUMERATOR,
      TRADING_FEE_DENOMINATOR,
      OWNER_TRADING_FEE_NUMERATOR,
      OWNER_TRADING_FEE_DENOMINATOR,
      OWNER_WITHDRAW_FEE_NUMERATOR,
      OWNER_WITHDRAW_FEE_DENOMINATOR,
      curveType,
      poolRegistryKey,
      poolNonce,
    );

    console.log('loading token swap');
    const fetchedTokenSwap = await TokenSwap.loadTokenSwap(
      connection,
      tokenSwapKey,
      STEP_SWAP_PROGRAM_ID,
      payer.publicKey,
      payer,
    );

    const poolRegistry = await TokenSwap.loadPoolRegistry(
      connection,
      payer.publicKey,
      STEP_SWAP_PROGRAM_ID,
    );

    if (!poolRegistry) {
      assert(poolRegistry !== undefined);
      return false;
    }

    assert(poolRegistry.isInitialized);
    assert(poolRegistry.registrySize > 1);
    assert(
      poolRegistry.accounts[poolRegistry.registrySize - 1].equals(tokenSwapKey),
    );

    assert(fetchedTokenSwap.tokenProgramId.equals(TOKEN_PROGRAM_ID));
    assert(fetchedTokenSwap.tokenAccountA.equals(tokenAccountA));
    assert(fetchedTokenSwap.tokenAccountB.equals(tokenAccountB));
    assert(fetchedTokenSwap.mintA.equals(mintA.publicKey));
    assert(fetchedTokenSwap.mintB.equals(mintB.publicKey));
    assert(fetchedTokenSwap.poolToken.equals(tokenPool.publicKey));
    assert(fetchedTokenSwap.feeAccount.equals(feeAccount));
    assert(
      TRADING_FEE_NUMERATOR == fetchedTokenSwap.tradeFeeNumerator.toNumber(),
    );
    assert(
      TRADING_FEE_DENOMINATOR ==
        fetchedTokenSwap.tradeFeeDenominator.toNumber(),
    );
    assert(
      OWNER_TRADING_FEE_NUMERATOR ==
        fetchedTokenSwap.ownerTradeFeeNumerator.toNumber(),
    );
    assert(
      OWNER_TRADING_FEE_DENOMINATOR ==
        fetchedTokenSwap.ownerTradeFeeDenominator.toNumber(),
    );
    assert(
      OWNER_WITHDRAW_FEE_NUMERATOR ==
        fetchedTokenSwap.ownerWithdrawFeeNumerator.toNumber(),
    );
    assert(
      OWNER_WITHDRAW_FEE_DENOMINATOR ==
        fetchedTokenSwap.ownerWithdrawFeeDenominator.toNumber(),
    );
    assert(curveType == fetchedTokenSwap.curveType);
    assert(poolNonce == fetchedTokenSwap.poolNonce);

    return true;
  } catch {}

  return false;
}

export async function createSecondTokenSwapForRouting(): Promise<void> {
  const connection = await getConnection();

  console.log('creating token C');
  mintC = await Token.createMint(
    connection,
    payer,
    owner.publicKey,
    null,
    2,
    TOKEN_PROGRAM_ID,
  );

  const seedKeyVec = [mintB.publicKey.toBuffer(), mintC.publicKey.toBuffer()];
  seedKeyVec.sort();

  const curveBuffer = Buffer.alloc(1);
  curveBuffer.writeUInt8(CURVE_TYPE, 0);
  let [tokenSwapKey, poolNonce] = await PublicKey.findProgramAddress(
    [seedKeyVec[0], seedKeyVec[1], curveBuffer],
    STEP_SWAP_PROGRAM_ID,
  );

  [authority2, nonce2] = await PublicKey.findProgramAddress(
    [tokenSwapKey.toBuffer()],
    STEP_SWAP_PROGRAM_ID,
  );

  console.log('creating pool mint');
  tokenPool2 = await Token.createMint(
    connection,
    payer,
    authority2,
    null,
    2,
    TOKEN_PROGRAM_ID,
  );

  console.log('creating pool account');
  tokenAccountPool2 = await tokenPool2.createAccount(owner.publicKey);
  const ownerKey = SWAP_PROGRAM_OWNER_FEE_ADDRESS || owner.publicKey.toString();
  feeAccount2 = await tokenPool2.createAccount(new PublicKey(ownerKey));

  console.log('creating token B account');
  tokenAccountB2 = await mintB.createAccount(authority2);
  console.log('minting token B to swap');
  await mintB.mintTo(tokenAccountB2, owner, [], currentSwapTokenB2);

  console.log('creating token C account');
  tokenAccountC = await mintC.createAccount(authority2);
  console.log('minting token C to swap');
  await mintC.mintTo(tokenAccountC, owner, [], currentSwapTokenC);

  const poolRegistryKey = await PublicKey.createWithSeed(
    payer.publicKey,
    POOL_REGISTRY_SEED,
    STEP_SWAP_PROGRAM_ID,
  );

  console.log('creating token swap');
  tokenSwap2 = await TokenSwap.createTokenSwap(
    connection,
    owner,
    tokenSwapKey,
    authority2,
    tokenAccountB2,
    tokenAccountC,
    tokenPool2.publicKey,
    mintB.publicKey,
    mintC.publicKey,
    feeAccount2,
    tokenAccountPool2,
    STEP_SWAP_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    nonce2,
    TRADING_FEE_NUMERATOR,
    TRADING_FEE_DENOMINATOR,
    OWNER_TRADING_FEE_NUMERATOR,
    OWNER_TRADING_FEE_DENOMINATOR,
    OWNER_WITHDRAW_FEE_NUMERATOR,
    OWNER_WITHDRAW_FEE_DENOMINATOR,
    CURVE_TYPE,
    poolRegistryKey,
    poolNonce,
  );
}

/// mint b -> native
export async function createThirdTokenSwapForNative(): Promise<void> {
  const connection = await getConnection();

  const seedKeyVec = [mintB.publicKey.toBuffer(), mintNativePubkey.toBuffer()];
  seedKeyVec.sort();

  const curveBuffer = Buffer.alloc(1);
  curveBuffer.writeUInt8(CURVE_TYPE, 0);
  let [tokenSwapKey, poolNonce] = await PublicKey.findProgramAddress(
    [seedKeyVec[0], seedKeyVec[1], curveBuffer],
    STEP_SWAP_PROGRAM_ID,
  );

  [authority3, nonce3] = await PublicKey.findProgramAddress(
    [tokenSwapKey.toBuffer()],
    STEP_SWAP_PROGRAM_ID,
  );

  console.log('creating pool mint for native pool');
  tokenPool3 = await Token.createMint(
    connection,
    payer,
    authority3,
    null,
    2,
    TOKEN_PROGRAM_ID,
  );

  console.log('creating pool account for native');
  tokenAccountPool3 = await tokenPool3.createAccount(owner.publicKey);
  const ownerKey = SWAP_PROGRAM_OWNER_FEE_ADDRESS || owner.publicKey.toString();
  feeAccount3 = await tokenPool3.createAccount(new PublicKey(ownerKey));

  console.log('creating token B account');
  tokenAccountANativeSwap = await mintB.createAccount(authority3);
  console.log('minting token B to swap');
  await mintB.mintTo(
    tokenAccountANativeSwap,
    owner,
    [],
    currentSwapTokenANative,
  );

  console.log('creating token native account');
  tokenAccountBNativeSwap = await Token.createWrappedNativeAccount(
    connection,
    TOKEN_PROGRAM_ID,
    authority3,
    owner,
    currentSwapTokenBNative,
  );

  const poolRegistryKey = await PublicKey.createWithSeed(
    payer.publicKey,
    POOL_REGISTRY_SEED,
    STEP_SWAP_PROGRAM_ID,
  );

  console.log('creating token swap');
  tokenSwap3 = await TokenSwap.createTokenSwap(
    connection,
    owner,
    tokenSwapKey,
    authority3,
    tokenAccountANativeSwap,
    tokenAccountBNativeSwap,
    tokenPool3.publicKey,
    mintB.publicKey,
    mintNativePubkey,
    feeAccount3,
    tokenAccountPool3,
    STEP_SWAP_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    nonce3,
    TRADING_FEE_NUMERATOR,
    TRADING_FEE_DENOMINATOR,
    OWNER_TRADING_FEE_NUMERATOR,
    OWNER_TRADING_FEE_DENOMINATOR,
    OWNER_WITHDRAW_FEE_NUMERATOR,
    OWNER_WITHDRAW_FEE_DENOMINATOR,
    CURVE_TYPE,
    poolRegistryKey,
    poolNonce,
  );
}

export async function depositAllTokenTypes(): Promise<void> {
  const poolMintInfo = await tokenPool.getMintInfo();
  const supply = poolMintInfo.supply.toNumber();
  const swapTokenA = await mintA.getAccountInfo(tokenAccountA);
  const tokenA = Math.floor(
    (swapTokenA.amount.toNumber() * POOL_TOKEN_AMOUNT) / supply,
  );
  const swapTokenB = await mintB.getAccountInfo(tokenAccountB);
  const tokenB = Math.floor(
    (swapTokenB.amount.toNumber() * POOL_TOKEN_AMOUNT) / supply,
  );

  const userTransferAuthority = Keypair.generate();
  console.log('Creating depositor token a account');
  const userAccountA = await mintA.createAccount(owner.publicKey);
  await mintA.mintTo(userAccountA, owner, [], tokenA);
  await mintA.approve(
    userAccountA,
    userTransferAuthority.publicKey,
    owner,
    [],
    tokenA,
  );
  console.log('Creating depositor token b account');
  const userAccountB = await mintB.createAccount(owner.publicKey);
  await mintB.mintTo(userAccountB, owner, [], tokenB);
  await mintB.approve(
    userAccountB,
    userTransferAuthority.publicKey,
    owner,
    [],
    tokenB,
  );
  console.log('Creating depositor pool token account');
  const newAccountPool = await tokenPool.createAccount(owner.publicKey);

  console.log('Depositing into swap');
  await tokenSwap.depositAllTokenTypes(
    userAccountA,
    userAccountB,
    newAccountPool,
    userTransferAuthority,
    POOL_TOKEN_AMOUNT,
    tokenA,
    tokenB,
  );

  let info;
  info = await mintA.getAccountInfo(userAccountA);
  assert(info.amount.toNumber() == 0);
  info = await mintB.getAccountInfo(userAccountB);
  assert(info.amount.toNumber() == 0);
  info = await mintA.getAccountInfo(tokenAccountA);
  assert(info.amount.toNumber() == currentSwapTokenA + tokenA);
  currentSwapTokenA += tokenA;
  info = await mintB.getAccountInfo(tokenAccountB);
  assert(info.amount.toNumber() == currentSwapTokenB + tokenB);
  currentSwapTokenB += tokenB;
  info = await tokenPool.getAccountInfo(newAccountPool);
  assert(info.amount.toNumber() == POOL_TOKEN_AMOUNT);
}

export async function withdrawAllTokenTypes(): Promise<void> {
  const poolMintInfo = await tokenPool.getMintInfo();
  const supply = poolMintInfo.supply.toNumber();
  let swapTokenA = await mintA.getAccountInfo(tokenAccountA);
  let swapTokenB = await mintB.getAccountInfo(tokenAccountB);
  let feeAmount = 0;
  if (OWNER_WITHDRAW_FEE_NUMERATOR !== 0) {
    feeAmount = Math.floor(
      (POOL_TOKEN_AMOUNT * OWNER_WITHDRAW_FEE_NUMERATOR) /
        OWNER_WITHDRAW_FEE_DENOMINATOR,
    );
  }
  const poolTokenAmount = POOL_TOKEN_AMOUNT - feeAmount;
  const tokenA = Math.floor(
    (swapTokenA.amount.toNumber() * poolTokenAmount) / supply,
  );
  const tokenB = Math.floor(
    (swapTokenB.amount.toNumber() * poolTokenAmount) / supply,
  );

  console.log('Creating withdraw token A account');
  let userAccountA = await mintA.createAccount(owner.publicKey);
  console.log('Creating withdraw token B account');
  let userAccountB = await mintB.createAccount(owner.publicKey);

  const userTransferAuthority = Keypair.generate();
  console.log('Approving withdrawal from pool account');
  await tokenPool.approve(
    tokenAccountPool,
    userTransferAuthority.publicKey,
    owner,
    [],
    POOL_TOKEN_AMOUNT,
  );

  console.log('Withdrawing pool tokens for A and B tokens');
  await tokenSwap.withdrawAllTokenTypes(
    userAccountA,
    userAccountB,
    tokenAccountPool,
    userTransferAuthority,
    POOL_TOKEN_AMOUNT,
    tokenA,
    tokenB,
  );

  //const poolMintInfo = await tokenPool.getMintInfo();
  swapTokenA = await mintA.getAccountInfo(tokenAccountA);
  swapTokenB = await mintB.getAccountInfo(tokenAccountB);

  let info = await tokenPool.getAccountInfo(tokenAccountPool);
  assert(
    info.amount.toNumber() == DEFAULT_POOL_TOKEN_AMOUNT - POOL_TOKEN_AMOUNT,
  );
  assert(swapTokenA.amount.toNumber() == currentSwapTokenA - tokenA);
  currentSwapTokenA -= tokenA;
  assert(swapTokenB.amount.toNumber() == currentSwapTokenB - tokenB);
  currentSwapTokenB -= tokenB;
  info = await mintA.getAccountInfo(userAccountA);
  assert(info.amount.toNumber() == tokenA);
  info = await mintB.getAccountInfo(userAccountB);
  assert(info.amount.toNumber() == tokenB);
  info = await tokenPool.getAccountInfo(feeAccount);
  assert(info.amount.toNumber() == feeAmount);
  currentFeeAmount = feeAmount;
}

export async function createAccountAndSwapAtomic(): Promise<void> {
  console.log('Creating swap token a account');
  let userAccountA = await mintA.createAccount(owner.publicKey);
  await mintA.mintTo(userAccountA, owner, [], SWAP_AMOUNT_IN);

  // @ts-ignore
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const newAccount = Keypair.generate();
  const transaction = new Transaction();
  transaction.add(
    SystemProgram.createAccount({
      fromPubkey: owner.publicKey,
      newAccountPubkey: newAccount.publicKey,
      lamports: balanceNeeded,
      space: AccountLayout.span,
      programId: mintB.programId,
    }),
  );

  transaction.add(
    Token.createInitAccountInstruction(
      mintB.programId,
      mintB.publicKey,
      newAccount.publicKey,
      owner.publicKey,
    ),
  );

  const userTransferAuthority = Keypair.generate();
  transaction.add(
    Token.createApproveInstruction(
      mintA.programId,
      userAccountA,
      userTransferAuthority.publicKey,
      owner.publicKey,
      [owner],
      SWAP_AMOUNT_IN,
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
      owner.publicKey,
      tokenSwap.swapProgramId,
      tokenSwap.tokenProgramId,
      SWAP_AMOUNT_IN,
      0,
    ),
  );

  // Send the instructions
  console.log('sending big instruction');
  await sendAndConfirmTransaction(
    'create account, approve transfer, swap',
    connection,
    transaction,
    owner,
    newAccount,
    userTransferAuthority,
  );

  let info;
  info = await mintA.getAccountInfo(tokenAccountA);
  currentSwapTokenA = info.amount.toNumber();
  info = await mintB.getAccountInfo(tokenAccountB);
  currentSwapTokenB = info.amount.toNumber();
}

export async function swap(): Promise<void> {
  console.log('Creating swap token a account');
  let userAccountA = await mintA.createAccount(owner.publicKey);
  await mintA.mintTo(userAccountA, owner, [], SWAP_AMOUNT_IN);
  const userTransferAuthority = Keypair.generate();
  await mintA.approve(
    userAccountA,
    userTransferAuthority.publicKey,
    owner,
    [],
    SWAP_AMOUNT_IN,
  );
  console.log('Creating swap token b account');
  let userAccountB = await mintB.createAccount(owner.publicKey);

  console.log('Swapping');
  await tokenSwap.swap(
    userAccountA,
    tokenAccountA,
    tokenAccountB,
    userAccountB,
    userTransferAuthority,
    owner.publicKey,
    SWAP_AMOUNT_IN,
    SWAP_AMOUNT_OUT,
  );

  await sleep(500);

  let info;
  info = await mintA.getAccountInfo(userAccountA);
  assert(info.amount.toNumber() == 0);

  info = await mintB.getAccountInfo(userAccountB);
  assert(info.amount.toNumber() == SWAP_AMOUNT_OUT);

  info = await mintA.getAccountInfo(tokenAccountA);
  assert(info.amount.toNumber() == currentSwapTokenA + SWAP_AMOUNT_IN);
  currentSwapTokenA += SWAP_AMOUNT_IN;

  info = await mintB.getAccountInfo(tokenAccountB);
  assert(info.amount.toNumber() == currentSwapTokenB - SWAP_AMOUNT_OUT);
  currentSwapTokenB -= SWAP_AMOUNT_OUT;

  info = await tokenPool.getAccountInfo(tokenAccountPool);
  assert(
    info.amount.toNumber() == DEFAULT_POOL_TOKEN_AMOUNT - POOL_TOKEN_AMOUNT,
  );

  info = await tokenPool.getAccountInfo(feeAccount);
  assert(info.amount.toNumber() == currentFeeAmount + OWNER_SWAP_FEE);
}

export async function routedSwap(): Promise<void> {
  console.log('Creating swap token a account');
  let userAccountA = await mintA.createAccount(owner.publicKey);
  await mintA.mintTo(userAccountA, owner, [], SWAP_AMOUNT_IN);
  const userTransferAuthority = Keypair.generate();
  await mintA.approve(
    userAccountA,
    userTransferAuthority.publicKey,
    owner,
    [],
    SWAP_AMOUNT_IN,
  );
  console.log('Creating swap token b account');
  let userAccountB = await mintB.createAccount(userTransferAuthority.publicKey);
  console.log('Creating swap token c account');
  let userAccountC = await mintC.createAccount(owner.publicKey);

  console.log(
    'userTransferAuthority',
    userTransferAuthority.publicKey.toString(),
  );
  console.log('owner.publicKey', owner.publicKey.toString());
  console.log('Swapping');
  await tokenSwap.routedSwap(
    userAccountA,
    tokenAccountA,
    tokenAccountB,
    userAccountB,
    tokenAccountB2,
    tokenAccountC,
    userAccountC,
    userTransferAuthority,
    owner.publicKey,
    tokenSwap2,
    SWAP_AMOUNT_IN,
    ROUTED_SWAP_AMOUNT_OUT2,
  );

  await sleep(500);

  let info;
  info = await mintA.getAccountInfo(userAccountA);
  assert(info.amount.toNumber() == 0);

  try {
    await mintB.getAccountInfo(userAccountB);
    assert(false, 'user intermediary account should not exist');
  } catch {}

  info = await mintC.getAccountInfo(userAccountC);
  console.log('ROUTED_SWAP_AMOUNT_OUT2', info.amount.toNumber());
  assert(info.amount.toNumber() == ROUTED_SWAP_AMOUNT_OUT2);

  info = await mintA.getAccountInfo(tokenAccountA);
  assert(info.amount.toNumber() == currentSwapTokenA + SWAP_AMOUNT_IN);
  currentSwapTokenA += SWAP_AMOUNT_IN;

  info = await mintB.getAccountInfo(tokenAccountB);
  assert(info.amount.toNumber() == currentSwapTokenB - ROUTED_SWAP_AMOUNT_OUT1);
  currentSwapTokenB -= ROUTED_SWAP_AMOUNT_OUT1;

  info = await mintB.getAccountInfo(tokenAccountB2);
  console.log(
    'ROUTED_SWAP_AMOUNT_OUT1',
    info.amount.toNumber() - currentSwapTokenB2,
  );
  assert(
    info.amount.toNumber() == currentSwapTokenB2 + ROUTED_SWAP_AMOUNT_OUT1,
  );
  currentSwapTokenB2 += ROUTED_SWAP_AMOUNT_OUT1;

  info = await mintC.getAccountInfo(tokenAccountC);
  assert(info.amount.toNumber() == currentSwapTokenC - ROUTED_SWAP_AMOUNT_OUT2);
  currentSwapTokenC -= ROUTED_SWAP_AMOUNT_OUT2;

  info = await tokenPool.getAccountInfo(feeAccount);
  assert(info.amount.toNumber() == OWNER_ROUTED_SWAP_FEE1);

  info = await tokenPool2.getAccountInfo(feeAccount2);
  assert(info.amount.toNumber() == OWNER_ROUTED_SWAP_FEE2);
}

//similar to the other "all-in-one" test, this test has no assertions
//it can be referred to as best practice for doing a routed swap.
export async function createAccountsAndRoutedSwapAtomic(): Promise<void> {
  console.log('Creating swap token a account');
  let userAccountA = await mintA.createAccount(owner.publicKey);
  await mintA.mintTo(userAccountA, owner, [], SWAP_AMOUNT_IN);

  // @ts-ignore
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );

  //use a temp account for the middle(B) token, even if user already has some, this is safest
  const newAccountB = Keypair.generate();

  //find ata address for C
  let newAccountC = await Token.getAssociatedTokenAddress(
    mintC.associatedProgramId,
    mintC.programId,
    mintC.publicKey,
    owner.publicKey,
  );

  const transaction = new Transaction();

  //create temp account for token B
  transaction.add(
    SystemProgram.createAccount({
      fromPubkey: owner.publicKey,
      newAccountPubkey: newAccountB.publicKey,
      lamports: balanceNeeded,
      space: AccountLayout.span,
      programId: mintB.programId,
    }),
  );

  //create a temp authority for A and B to use
  const userTransferAuthority = Keypair.generate();

  //init B token account, assign to temp authority
  transaction.add(
    Token.createInitAccountInstruction(
      mintB.programId,
      mintB.publicKey,
      newAccountB.publicKey,
      userTransferAuthority.publicKey,
    ),
  );

  //init ATA for token C
  transaction.add(
    Token.createAssociatedTokenAccountInstruction(
      mintC.associatedProgramId,
      mintC.programId,
      mintC.publicKey,
      newAccountC,
      owner.publicKey,
      owner.publicKey,
    ),
  );

  //temp authority to spend token A from owner wallet
  transaction.add(
    Token.createApproveInstruction(
      mintA.programId,
      userAccountA,
      userTransferAuthority.publicKey,
      owner.publicKey,
      [owner],
      SWAP_AMOUNT_IN,
    ),
  );

  //swap
  transaction.add(
    TokenSwap.routedSwapInstruction(
      tokenSwap.tokenSwap,
      tokenSwap.authority,
      userTransferAuthority.publicKey,
      userAccountA,
      tokenSwap.tokenAccountA,
      tokenSwap.tokenAccountB,
      newAccountB.publicKey,
      tokenSwap.poolToken,
      tokenSwap.feeAccount,
      tokenSwap2.tokenSwap,
      tokenSwap2.authority,
      tokenSwap2.tokenAccountA,
      tokenSwap2.tokenAccountB,
      newAccountC,
      tokenSwap2.poolToken,
      tokenSwap2.feeAccount,
      owner.publicKey,
      tokenSwap.swapProgramId,
      tokenSwap.tokenProgramId,
      SWAP_AMOUNT_IN,
      0, //apps should set this for sure, but in this test can't be ROUTED_SWAP_AMOUNT_OUT anymore because we already swapped some
    ),
  );

  console.log('before swap');
  console.log(
    'userAccountA',
    (await mintA.getAccountInfo(userAccountA)).amount.toNumber(),
  );

  // Send the instructions
  console.log('sending biggest instruction');
  await sendAndConfirmTransaction(
    'create accounts, approve transfer, swap, cleanup',
    connection,
    transaction,
    owner,
    newAccountB,
    userTransferAuthority,
  );

  console.log('after swap');
  console.log(
    'userAccountA',
    (await mintA.getAccountInfo(userAccountA)).amount.toNumber(),
  );
  console.log(
    'newAccountC',
    (await mintC.getAccountInfo(newAccountC)).amount.toNumber(),
  );

  let info;
  info = await mintA.getAccountInfo(tokenAccountA);
  currentSwapTokenA = info.amount.toNumber();
  info = await mintB.getAccountInfo(tokenAccountB);
  currentSwapTokenB = info.amount.toNumber();
  info = await mintC.getAccountInfo(tokenAccountC);
  currentSwapTokenC = info.amount.toNumber();
}

function tradingTokensToPoolTokens(
  sourceAmount: number,
  swapSourceAmount: number,
  poolAmount: number,
): number {
  const tradingFee =
    (sourceAmount / 2) * (TRADING_FEE_NUMERATOR / TRADING_FEE_DENOMINATOR);
  const sourceAmountPostFee = sourceAmount - tradingFee;
  const root = Math.sqrt(sourceAmountPostFee / swapSourceAmount + 1);
  return Math.floor(poolAmount * (root - 1));
}

export async function swapNative(): Promise<void> {
  console.log('Creating swap token b account');
  let userAccountA = await mintB.createAccount(owner.publicKey);
  await mintB.mintTo(userAccountA, owner, [], SWAP_AMOUNT_IN * 3);

  {
    const userTransferAuthority = Keypair.generate();
    await mintB.approve(
      userAccountA,
      userTransferAuthority.publicKey,
      owner,
      [],
      SWAP_AMOUNT_IN,
    );
    console.log('Creating swap token native account');
    let userAccountNative = await Token.createWrappedNativeAccount(
      connection,
      TOKEN_PROGRAM_ID,
      owner.publicKey,
      owner,
      currentSwapTokenC,
    );

    let balance = await connection.getBalance(owner.publicKey);
    console.log('balance', balance);

    console.log('Swapping');
    await tokenSwap3.swap(
      userAccountA,
      tokenAccountANativeSwap,
      tokenAccountBNativeSwap,
      userAccountNative,
      userTransferAuthority,
      owner.publicKey,
      SWAP_AMOUNT_IN,
      0,
    );

    await sleep(500);

    let newBalance = await connection.getBalance(owner.publicKey);
    console.log('newBalance', newBalance);
    let nativeSwapAmount = newBalance - balance;
    console.log('nativeSwapAmount', nativeSwapAmount);
    //not testing the math, just the functionality
    assert(nativeSwapAmount < 0, 'lamports of refunder increased');

    //verify native sol account not closed
    let nativeAccount = await connection.getAccountInfo(userAccountNative);
    assert(nativeAccount != null, 'wsol account closed');
  }

  //now do it again with the user transfer auth owning the wsol output
  {
    const userTransferAuthority = Keypair.generate();
    await mintB.approve(
      userAccountA,
      userTransferAuthority.publicKey,
      owner,
      [],
      SWAP_AMOUNT_IN,
    );
    console.log('Creating swap token native account');
    let userAccountNative = await Token.createWrappedNativeAccount(
      connection,
      TOKEN_PROGRAM_ID,
      userTransferAuthority.publicKey,
      owner,
      currentSwapTokenC,
    );

    let balance = await connection.getBalance(owner.publicKey);
    console.log('balance', balance);

    console.log('Swapping');
    await tokenSwap3.swap(
      userAccountA,
      tokenAccountANativeSwap,
      tokenAccountBNativeSwap,
      userAccountNative,
      userTransferAuthority,
      owner.publicKey,
      SWAP_AMOUNT_IN,
      0,
    );

    await sleep(500);

    let newBalance = await connection.getBalance(owner.publicKey);
    console.log('newBalance', newBalance);
    let nativeSwapAmount = newBalance - balance;
    console.log('nativeSwapAmount', nativeSwapAmount);
    //not testing the math, just the functionality
    assert(nativeSwapAmount > 100, 'lamports of refunder did not increase');

    //verify native sol account closed
    let nativeAccount = await connection.getAccountInfo(userAccountNative);
    assert(nativeAccount == null, 'wsol account not closed');
  }

  //now do it again with the user transfer auth owning the wsol output
  //but flags specifying NOT to close
  {
    await mintB.approve(
      userAccountA,
      owner.publicKey,
      owner,
      [],
      SWAP_AMOUNT_IN,
    );
    console.log('Creating swap token native account');
    let userAccountNative = await Token.createWrappedNativeAccount(
      connection,
      TOKEN_PROGRAM_ID,
      owner.publicKey,
      owner,
      currentSwapTokenC,
    );

    let balance = await connection.getBalance(owner.publicKey);
    console.log('balance', balance);

    console.log('Swapping');
    await tokenSwap3.swap(
      userAccountA,
      tokenAccountANativeSwap,
      tokenAccountBNativeSwap,
      userAccountNative,
      owner,
      owner.publicKey,
      SWAP_AMOUNT_IN,
      0,
      0,
    );

    await sleep(500);

    let newBalance = await connection.getBalance(owner.publicKey);
    console.log('newBalance', newBalance);
    let nativeSwapAmount = newBalance - balance;
    console.log('nativeSwapAmount', nativeSwapAmount);
    //not testing the math, just the functionality
    assert(nativeSwapAmount < 0, 'lamports of refunder increased');

    //verify native sol account not closed
    let nativeAccount = await connection.getAccountInfo(userAccountNative);
    assert(nativeAccount != null, 'wsol account closed');
  }
}

export async function swapNativeToNewATALargeBoiTest(): Promise<void> {
  //create user transfer authority, also use as temp native account
  const userTransferAuthority = Keypair.generate();

  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );

  //use a temp account for the middle(B) token, even if user already has some, this is safest
  const newAccountB = Keypair.generate();

  //find ata address for C
  let newAccountC = await Token.getAssociatedTokenAddress(
    mintA.associatedProgramId,
    mintA.programId,
    mintA.publicKey,
    owner.publicKey,
  );

  const transaction = new Transaction();

  //create an token account to hold the native sol.
  //We use our already temporary throwaway authority to be the **account address**
  //the program will see this is temporary and delete this for us, no cleanup needed.
  transaction.add(
    SystemProgram.createAccount({
      fromPubkey: owner.publicKey,
      newAccountPubkey: userTransferAuthority.publicKey,
      lamports: balanceNeeded + SWAP_AMOUNT_IN,
      space: AccountLayout.span,
      programId: TOKEN_PROGRAM_ID,
    }),
  );
  //our throwaway authority is both the address AND the owner of this token
  transaction.add(
    Token.createInitAccountInstruction(
      TOKEN_PROGRAM_ID,
      NATIVE_MINT,
      userTransferAuthority.publicKey,
      userTransferAuthority.publicKey,
    ),
  );

  //create temp account for token B
  transaction.add(
    SystemProgram.createAccount({
      fromPubkey: owner.publicKey,
      newAccountPubkey: newAccountB.publicKey,
      lamports: balanceNeeded,
      space: AccountLayout.span,
      programId: mintB.programId,
    }),
  );
  //init B token account, assign ownership to temp authority
  //because we set the authority to the temp authority
  //the program will see this is temporary and delete this for us, no cleanup needed.
  transaction.add(
    Token.createInitAccountInstruction(
      mintB.programId,
      mintB.publicKey,
      newAccountB.publicKey,
      userTransferAuthority.publicKey,
    ),
  );

  //init ATA for token C
  transaction.add(
    Token.createAssociatedTokenAccountInstruction(
      mintA.associatedProgramId,
      mintA.programId,
      mintA.publicKey,
      newAccountC,
      owner.publicKey,
      owner.publicKey,
    ),
  );

  //swap
  transaction.add(
    TokenSwap.routedSwapInstruction(
      tokenSwap3.tokenSwap,
      tokenSwap3.authority,
      userTransferAuthority.publicKey,
      userTransferAuthority.publicKey,
      tokenSwap3.tokenAccountB,
      tokenSwap3.tokenAccountA,
      newAccountB.publicKey,
      tokenSwap3.poolToken,
      tokenSwap3.feeAccount,
      tokenSwap.tokenSwap,
      tokenSwap.authority,
      tokenSwap.tokenAccountB,
      tokenSwap.tokenAccountA,
      newAccountC,
      tokenSwap.poolToken,
      tokenSwap.feeAccount,
      owner.publicKey,
      tokenSwap.swapProgramId,
      tokenSwap.tokenProgramId,
      SWAP_AMOUNT_IN,
      0, //apps should set this for sure, but in this test can't be ROUTED_SWAP_AMOUNT_OUT anymore because we already swapped some
    ),
  );

  console.log('before swap');
  let solBalanceBefore = await connection.getBalance(owner.publicKey);
  console.log('userAccountA (users wallet)', solBalanceBefore);

  // Send the instructions
  console.log('sending biggest large boi instruction');
  await sendAndConfirmTransaction(
    'create accounts native, swap, cleanup',
    connection,
    transaction,
    owner,
    userTransferAuthority,
    newAccountB,
  );

  console.log('after swap');
  let solBalanceAfter = await connection.getBalance(owner.publicKey);
  console.log('userAccountA (users wallet)', solBalanceAfter);
  console.log(
    'newAccountC',
    (await mintA.getAccountInfo(newAccountC)).amount.toNumber(),
  );
}

export async function depositSingleTokenTypeExactAmountIn(): Promise<void> {
  // Pool token amount to deposit on one side
  const depositAmount = 10000;

  const poolMintInfo = await tokenPool.getMintInfo();
  const supply = poolMintInfo.supply.toNumber();
  const swapTokenA = await mintA.getAccountInfo(tokenAccountA);
  const poolTokenA = tradingTokensToPoolTokens(
    depositAmount,
    swapTokenA.amount.toNumber(),
    supply,
  );
  const swapTokenB = await mintB.getAccountInfo(tokenAccountB);
  const poolTokenB = tradingTokensToPoolTokens(
    depositAmount,
    swapTokenB.amount.toNumber(),
    supply,
  );

  const userTransferAuthority = Keypair.generate();
  console.log('Creating depositor token a account');
  const userAccountA = await mintA.createAccount(owner.publicKey);
  await mintA.mintTo(userAccountA, owner, [], depositAmount);
  await mintA.approve(
    userAccountA,
    userTransferAuthority.publicKey,
    owner,
    [],
    depositAmount,
  );
  console.log('Creating depositor token b account');
  const userAccountB = await mintB.createAccount(owner.publicKey);
  await mintB.mintTo(userAccountB, owner, [], depositAmount);
  await mintB.approve(
    userAccountB,
    userTransferAuthority.publicKey,
    owner,
    [],
    depositAmount,
  );
  console.log('Creating depositor pool token account');
  const newAccountPool = await tokenPool.createAccount(owner.publicKey);

  console.log('Depositing token A into swap');
  await tokenSwap.depositSingleTokenTypeExactAmountIn(
    userAccountA,
    newAccountPool,
    userTransferAuthority,
    depositAmount,
    poolTokenA,
  );

  let info;
  info = await mintA.getAccountInfo(userAccountA);
  assert(info.amount.toNumber() == 0);
  info = await mintA.getAccountInfo(tokenAccountA);
  assert(info.amount.toNumber() == currentSwapTokenA + depositAmount);
  currentSwapTokenA += depositAmount;

  console.log('Depositing token B into swap');
  await tokenSwap.depositSingleTokenTypeExactAmountIn(
    userAccountB,
    newAccountPool,
    userTransferAuthority,
    depositAmount,
    poolTokenB,
  );

  info = await mintB.getAccountInfo(userAccountB);
  assert(info.amount.toNumber() == 0);
  info = await mintB.getAccountInfo(tokenAccountB);
  assert(info.amount.toNumber() == currentSwapTokenB + depositAmount);
  currentSwapTokenB += depositAmount;
  info = await tokenPool.getAccountInfo(newAccountPool);
  assert(info.amount.toNumber() >= poolTokenA + poolTokenB);
}

export async function withdrawSingleTokenTypeExactAmountOut(): Promise<void> {
  // Pool token amount to withdraw on one side
  const withdrawAmount = 50000;
  const roundingAmount = 1.0001; // make math a little easier

  const poolMintInfo = await tokenPool.getMintInfo();
  const supply = poolMintInfo.supply.toNumber();

  const swapTokenA = await mintA.getAccountInfo(tokenAccountA);
  const swapTokenAPost = swapTokenA.amount.toNumber() - withdrawAmount;
  const poolTokenA = tradingTokensToPoolTokens(
    withdrawAmount,
    swapTokenAPost,
    supply,
  );
  let adjustedPoolTokenA = poolTokenA * roundingAmount;
  if (OWNER_WITHDRAW_FEE_NUMERATOR !== 0) {
    adjustedPoolTokenA *=
      1 + OWNER_WITHDRAW_FEE_NUMERATOR / OWNER_WITHDRAW_FEE_DENOMINATOR;
  }

  const swapTokenB = await mintB.getAccountInfo(tokenAccountB);
  const swapTokenBPost = swapTokenB.amount.toNumber() - withdrawAmount;
  const poolTokenB = tradingTokensToPoolTokens(
    withdrawAmount,
    swapTokenBPost,
    supply,
  );
  let adjustedPoolTokenB = poolTokenB * roundingAmount;
  if (OWNER_WITHDRAW_FEE_NUMERATOR !== 0) {
    adjustedPoolTokenB *=
      1 + OWNER_WITHDRAW_FEE_NUMERATOR / OWNER_WITHDRAW_FEE_DENOMINATOR;
  }

  const userTransferAuthority = Keypair.generate();
  console.log('Creating withdraw token a account');
  const userAccountA = await mintA.createAccount(owner.publicKey);
  console.log('Creating withdraw token b account');
  const userAccountB = await mintB.createAccount(owner.publicKey);
  console.log('Creating withdraw pool token account');
  const poolAccount = await tokenPool.getAccountInfo(tokenAccountPool);
  const poolTokenAmount = poolAccount.amount.toNumber();
  await tokenPool.approve(
    tokenAccountPool,
    userTransferAuthority.publicKey,
    owner,
    [],
    adjustedPoolTokenA + adjustedPoolTokenB,
  );

  console.log('Withdrawing token A only');
  await tokenSwap.withdrawSingleTokenTypeExactAmountOut(
    userAccountA,
    tokenAccountPool,
    userTransferAuthority,
    withdrawAmount,
    adjustedPoolTokenA,
  );

  let info;
  info = await mintA.getAccountInfo(userAccountA);
  assert(info.amount.toNumber() == withdrawAmount);
  info = await mintA.getAccountInfo(tokenAccountA);
  assert(info.amount.toNumber() == currentSwapTokenA - withdrawAmount);
  currentSwapTokenA += withdrawAmount;
  info = await tokenPool.getAccountInfo(tokenAccountPool);
  assert(info.amount.toNumber() >= poolTokenAmount - adjustedPoolTokenA);

  console.log('Withdrawing token B only');
  await tokenSwap.withdrawSingleTokenTypeExactAmountOut(
    userAccountB,
    tokenAccountPool,
    userTransferAuthority,
    withdrawAmount,
    adjustedPoolTokenB,
  );

  info = await mintB.getAccountInfo(userAccountB);
  assert(info.amount.toNumber() == withdrawAmount);
  info = await mintB.getAccountInfo(tokenAccountB);
  assert(info.amount.toNumber() == currentSwapTokenB - withdrawAmount);
  currentSwapTokenB += withdrawAmount;
  info = await tokenPool.getAccountInfo(tokenAccountPool);
  assert(
    info.amount.toNumber() >=
      poolTokenAmount - adjustedPoolTokenA - adjustedPoolTokenB,
  );
}
