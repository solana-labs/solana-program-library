var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import assert from 'assert';
import BN from 'bn.js';
import { Buffer } from 'buffer';
import * as BufferLayout from 'buffer-layout';
import { PublicKey, SystemProgram, Transaction, TransactionInstruction, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import * as Layout from './layout';
import { sendAndConfirmTransaction } from './util/send-and-confirm-transaction';
import { loadAccount } from './util/account';
export const STEP_SWAP_PROGRAM_ID = new PublicKey('SSwpMgqNDsyV7mAgN9ady4bDVu5ySjmmXejXvy2vLt1');
export const POOL_REGISTRY_SEED = "poolregistry";
/**
 * Some amount of tokens
 */
export class Numberu64 extends BN {
    /**
     * Convert to Buffer representation
     */
    toBuffer() {
        const a = super.toArray().reverse();
        const b = Buffer.from(a);
        if (b.length === 8) {
            return b;
        }
        assert(b.length < 8, 'Numberu64 too large');
        const zeroPad = Buffer.alloc(8);
        b.copy(zeroPad);
        return zeroPad;
    }
    /**
     * Construct a Numberu64 from Buffer representation
     */
    static fromBuffer(buffer) {
        assert(buffer.length === 8, `Invalid buffer length: ${buffer.length}`);
        return new Numberu64([...buffer]
            .reverse()
            .map(i => `00${i.toString(16)}`.slice(-2))
            .join(''), 16);
    }
}
class PublicKeyLayout extends BufferLayout.Blob {
    constructor(property) {
        super(32, property);
    }
    decode(b, offset) {
        return new PublicKey(super.decode(b, offset));
    }
    encode(src, b, offset) {
        return super.encode(src.toBuffer(), b, offset);
    }
}
function publicKeyLayout(property = "") {
    return new PublicKeyLayout(property);
}
export const PoolRegistryLayout = BufferLayout.struct([
    BufferLayout.u8('isInitialized'),
    BufferLayout.u32('registrySize'),
    BufferLayout.seq(publicKeyLayout(), ((2 * 1024 * 1024) / 32) - 1, "accounts"),
]);
export const TokenSwapLayout = BufferLayout.struct([
    BufferLayout.u8('version'),
    BufferLayout.u8('isInitialized'),
    BufferLayout.u8('nonce'),
    Layout.publicKey('tokenProgramId'),
    Layout.publicKey('tokenAccountA'),
    Layout.publicKey('tokenAccountB'),
    Layout.publicKey('tokenPool'),
    Layout.publicKey('mintA'),
    Layout.publicKey('mintB'),
    Layout.publicKey('feeAccount'),
    Layout.uint64('tradeFeeNumerator'),
    Layout.uint64('tradeFeeDenominator'),
    Layout.uint64('ownerTradeFeeNumerator'),
    Layout.uint64('ownerTradeFeeDenominator'),
    Layout.uint64('ownerWithdrawFeeNumerator'),
    Layout.uint64('ownerWithdrawFeeDenominator'),
    BufferLayout.u8('curveType'),
    BufferLayout.blob(32, 'curveParameters'),
    BufferLayout.u8('poolNonce'),
]);
export const CurveType = Object.freeze({
    ConstantProduct: 0,
    ConstantPrice: 1,
    Stable: 2,
    Offset: 3, // Offset curve, like Uniswap, but with an additional offset on the token B side
});
/**
 * A program to exchange tokens against a pool of liquidity
 */
export class TokenSwap {
    /**
     * Create a Token object attached to the specific token
     *
     * @param connection The connection to use
     * @param tokenSwap The token swap account
     * @param swapProgramId The program ID of the token-swap program
     * @param tokenProgramId The program ID of the token program
     * @param poolToken The pool token
     * @param authority The authority over the swap and accounts
     * @param tokenAccountA The token swap's Token A account
     * @param tokenAccountB The token swap's Token B account
     * @param mintA The mint of Token A
     * @param mintB The mint of Token B
     * @param tradeFeeNumerator The trade fee numerator
     * @param tradeFeeDenominator The trade fee denominator
     * @param ownerTradeFeeNumerator The owner trade fee numerator
     * @param ownerTradeFeeDenominator The owner trade fee denominator
     * @param ownerWithdrawFeeNumerator The owner withdraw fee numerator
     * @param ownerWithdrawFeeDenominator The owner withdraw fee denominator
     * @param curveType The curve type
     * @param payer Pays for the transaction
     * @param poolNonce Nonce for the swap PDA
     */
    constructor(connection, tokenSwap, swapProgramId, tokenProgramId, poolToken, feeAccount, authority, tokenAccountA, tokenAccountB, mintA, mintB, tradeFeeNumerator, tradeFeeDenominator, ownerTradeFeeNumerator, ownerTradeFeeDenominator, ownerWithdrawFeeNumerator, ownerWithdrawFeeDenominator, curveType, payer, poolRegistry, poolNonce) {
        this.connection = connection;
        this.tokenSwap = tokenSwap;
        this.swapProgramId = swapProgramId;
        this.tokenProgramId = tokenProgramId;
        this.poolToken = poolToken;
        this.feeAccount = feeAccount;
        this.authority = authority;
        this.tokenAccountA = tokenAccountA;
        this.tokenAccountB = tokenAccountB;
        this.mintA = mintA;
        this.mintB = mintB;
        this.tradeFeeNumerator = tradeFeeNumerator;
        this.tradeFeeDenominator = tradeFeeDenominator;
        this.ownerTradeFeeNumerator = ownerTradeFeeNumerator;
        this.ownerTradeFeeDenominator = ownerTradeFeeDenominator;
        this.ownerWithdrawFeeNumerator = ownerWithdrawFeeNumerator;
        this.ownerWithdrawFeeDenominator = ownerWithdrawFeeDenominator;
        this.curveType = curveType;
        this.payer = payer;
        this.poolRegistry = poolRegistry;
        this.poolNonce = poolNonce;
        this.connection = connection;
        this.tokenSwap = tokenSwap;
        this.swapProgramId = swapProgramId;
        this.tokenProgramId = tokenProgramId;
        this.poolToken = poolToken;
        this.feeAccount = feeAccount;
        this.authority = authority;
        this.tokenAccountA = tokenAccountA;
        this.tokenAccountB = tokenAccountB;
        this.mintA = mintA;
        this.mintB = mintB;
        this.tradeFeeNumerator = tradeFeeNumerator;
        this.tradeFeeDenominator = tradeFeeDenominator;
        this.ownerTradeFeeNumerator = ownerTradeFeeNumerator;
        this.ownerTradeFeeDenominator = ownerTradeFeeDenominator;
        this.ownerWithdrawFeeNumerator = ownerWithdrawFeeNumerator;
        this.ownerWithdrawFeeDenominator = ownerWithdrawFeeDenominator;
        this.curveType = curveType;
        this.payer = payer;
        this.poolRegistry = poolRegistry;
        this.poolNonce = poolNonce;
    }
    /**
     * Get the minimum balance for the token swap account to be rent exempt
     *
     * @return Number of lamports required
     */
    static getMinBalanceRentForExemptTokenSwap(connection) {
        return __awaiter(this, void 0, void 0, function* () {
            return yield connection.getMinimumBalanceForRentExemption(TokenSwapLayout.span);
        });
    }
    /**
     * Initialize the Pool Registry
     *
     * @param connection The connection to use
     * @param payer Pays for the transaction
     * @param swapProgramId The program ID of the token-swap program
     */
    static initializePoolRegistry(connection, registryOwner, swapProgramId) {
        return __awaiter(this, void 0, void 0, function* () {
            let transaction;
            // Allocate memory for the account
            const balanceNeeded = yield connection.getMinimumBalanceForRentExemption(PoolRegistryLayout.span);
            transaction = new Transaction();
            const poolRegistryKey = yield PublicKey.createWithSeed(registryOwner, POOL_REGISTRY_SEED, swapProgramId);
            transaction.add(SystemProgram.createAccountWithSeed({
                fromPubkey: registryOwner,
                newAccountPubkey: poolRegistryKey,
                basePubkey: registryOwner,
                seed: POOL_REGISTRY_SEED,
                lamports: balanceNeeded,
                space: PoolRegistryLayout.span,
                programId: swapProgramId,
            }));
            const instruction = TokenSwap.createInitRegistryInstruction(registryOwner, poolRegistryKey, swapProgramId);
            transaction.add(instruction);
            return transaction;
        });
    }
    /**
     * Loads the Pool Registry
     *
     * @param connection The connection to use
     * @param payer Pays for the transaction
     * @param swapProgramId The program ID of the token-swap program
     */
    static loadPoolRegistry(connection, registryOwner, swapProgramId) {
        return __awaiter(this, void 0, void 0, function* () {
            const poolRegistryKey = yield PublicKey.createWithSeed(registryOwner, POOL_REGISTRY_SEED, swapProgramId);
            const acc = yield connection.getAccountInfo(poolRegistryKey);
            if (!acc) {
                return undefined;
            }
            const decoded = PoolRegistryLayout.decode(acc.data);
            return {
                isInitialized: decoded.isInitialized,
                registrySize: decoded.registrySize,
                accounts: decoded.accounts
            };
        });
    }
    static createInitRegistryInstruction(registryOwner, poolRegistry, swapProgramId) {
        const keys = [
            { pubkey: registryOwner, isSigner: true, isWritable: false },
            { pubkey: poolRegistry, isSigner: false, isWritable: true },
        ];
        const dataLayout = BufferLayout.struct([
            BufferLayout.u8('instruction'),
        ]);
        const data = Buffer.alloc(dataLayout.span);
        dataLayout.encode({
            instruction: 6, // Init Registry instruction
        }, data);
        return new TransactionInstruction({
            keys,
            programId: swapProgramId,
            data,
        });
    }
    static createInitSwapInstruction(payer, tokenSwapKey, authority, tokenAccountA, tokenAccountB, tokenPool, feeAccount, tokenAccountPool, tokenProgramId, swapProgramId, nonce, tradeFeeNumerator, tradeFeeDenominator, ownerTradeFeeNumerator, ownerTradeFeeDenominator, ownerWithdrawFeeNumerator, ownerWithdrawFeeDenominator, curveType, poolRegistry, poolNonce) {
        const keys = [
            { pubkey: payer, isSigner: true, isWritable: false },
            { pubkey: tokenSwapKey, isSigner: false, isWritable: true },
            { pubkey: authority, isSigner: false, isWritable: false },
            { pubkey: tokenAccountA, isSigner: false, isWritable: false },
            { pubkey: tokenAccountB, isSigner: false, isWritable: false },
            { pubkey: tokenPool, isSigner: false, isWritable: true },
            { pubkey: feeAccount, isSigner: false, isWritable: false },
            { pubkey: tokenAccountPool, isSigner: false, isWritable: true },
            { pubkey: tokenProgramId, isSigner: false, isWritable: false },
            { pubkey: poolRegistry, isSigner: false, isWritable: true },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
        ];
        const commandDataLayout = BufferLayout.struct([
            BufferLayout.u8('instruction'),
            BufferLayout.u8('nonce'),
            BufferLayout.nu64('tradeFeeNumerator'),
            BufferLayout.nu64('tradeFeeDenominator'),
            BufferLayout.nu64('ownerTradeFeeNumerator'),
            BufferLayout.nu64('ownerTradeFeeDenominator'),
            BufferLayout.nu64('ownerWithdrawFeeNumerator'),
            BufferLayout.nu64('ownerWithdrawFeeDenominator'),
            BufferLayout.u8('curveType'),
            BufferLayout.blob(32, 'curveParameters'),
            BufferLayout.u8('poolNonce'),
        ]);
        let data = Buffer.alloc(1024);
        {
            const encodeLength = commandDataLayout.encode({
                instruction: 0,
                nonce,
                tradeFeeNumerator,
                tradeFeeDenominator,
                ownerTradeFeeNumerator,
                ownerTradeFeeDenominator,
                ownerWithdrawFeeNumerator,
                ownerWithdrawFeeDenominator,
                curveType,
                poolNonce
            }, data);
            data = data.slice(0, encodeLength);
        }
        return new TransactionInstruction({
            keys,
            programId: swapProgramId,
            data,
        });
    }
    static loadTokenSwap(connection, address, programId, registryOwner, payer) {
        return __awaiter(this, void 0, void 0, function* () {
            const data = yield loadAccount(connection, address, programId);
            const tokenSwapData = TokenSwapLayout.decode(data);
            if (!tokenSwapData.isInitialized) {
                throw new Error(`Invalid token swap state`);
            }
            const poolRegistry = yield PublicKey.createWithSeed(registryOwner, POOL_REGISTRY_SEED, programId);
            const [authority] = yield PublicKey.findProgramAddress([address.toBuffer()], programId);
            const poolToken = new PublicKey(tokenSwapData.tokenPool);
            const feeAccount = new PublicKey(tokenSwapData.feeAccount);
            const tokenAccountA = new PublicKey(tokenSwapData.tokenAccountA);
            const tokenAccountB = new PublicKey(tokenSwapData.tokenAccountB);
            const mintA = new PublicKey(tokenSwapData.mintA);
            const mintB = new PublicKey(tokenSwapData.mintB);
            const tokenProgramId = new PublicKey(tokenSwapData.tokenProgramId);
            const tradeFeeNumerator = Numberu64.fromBuffer(tokenSwapData.tradeFeeNumerator);
            const tradeFeeDenominator = Numberu64.fromBuffer(tokenSwapData.tradeFeeDenominator);
            const ownerTradeFeeNumerator = Numberu64.fromBuffer(tokenSwapData.ownerTradeFeeNumerator);
            const ownerTradeFeeDenominator = Numberu64.fromBuffer(tokenSwapData.ownerTradeFeeDenominator);
            const ownerWithdrawFeeNumerator = Numberu64.fromBuffer(tokenSwapData.ownerWithdrawFeeNumerator);
            const ownerWithdrawFeeDenominator = Numberu64.fromBuffer(tokenSwapData.ownerWithdrawFeeDenominator);
            const curveType = tokenSwapData.curveType;
            const poolNonce = tokenSwapData.poolNonce;
            return new TokenSwap(connection, address, programId, tokenProgramId, poolToken, feeAccount, authority, tokenAccountA, tokenAccountB, mintA, mintB, tradeFeeNumerator, tradeFeeDenominator, ownerTradeFeeNumerator, ownerTradeFeeDenominator, ownerWithdrawFeeNumerator, ownerWithdrawFeeDenominator, curveType, payer, poolRegistry, poolNonce);
        });
    }
    /**
     * Create a new Token Swap
     *
     * @param connection The connection to use
     * @param payer Pays for the transaction
     * @param tokenSwapAccount The token swap account
     * @param authority The authority over the swap and accounts
     * @param nonce The nonce used to generate the authority
     * @param tokenAccountA: The token swap's Token A account
     * @param tokenAccountB: The token swap's Token B account
     * @param poolToken The pool token
     * @param tokenAccountPool The token swap's pool token account
     * @param tokenProgramId The program ID of the token program
     * @param swapProgramId The program ID of the token-swap program
     * @param feeNumerator Numerator of the fee ratio
     * @param feeDenominator Denominator of the fee ratio
     * @return Token object for the newly minted token, Public key of the account holding the total supply of new tokens
     */
    static createTokenSwap(connection, payer, tokenSwapKey, authority, tokenAccountA, tokenAccountB, poolToken, mintA, mintB, feeAccount, tokenAccountPool, swapProgramId, tokenProgramId, nonce, tradeFeeNumerator, tradeFeeDenominator, ownerTradeFeeNumerator, ownerTradeFeeDenominator, ownerWithdrawFeeNumerator, ownerWithdrawFeeDenominator, curveType, poolRegistry, poolNonce) {
        return __awaiter(this, void 0, void 0, function* () {
            let transaction;
            const tokenSwap = new TokenSwap(connection, tokenSwapKey, swapProgramId, tokenProgramId, poolToken, feeAccount, authority, tokenAccountA, tokenAccountB, mintA, mintB, new Numberu64(tradeFeeNumerator), new Numberu64(tradeFeeDenominator), new Numberu64(ownerTradeFeeNumerator), new Numberu64(ownerTradeFeeDenominator), new Numberu64(ownerWithdrawFeeNumerator), new Numberu64(ownerWithdrawFeeDenominator), curveType, payer, poolRegistry, poolNonce);
            // Allocate memory for the account
            const balanceNeeded = yield TokenSwap.getMinBalanceRentForExemptTokenSwap(connection);
            transaction = new Transaction();
            const instruction = TokenSwap.createInitSwapInstruction(payer.publicKey, tokenSwapKey, authority, tokenAccountA, tokenAccountB, poolToken, feeAccount, tokenAccountPool, tokenProgramId, swapProgramId, nonce, tradeFeeNumerator, tradeFeeDenominator, ownerTradeFeeNumerator, ownerTradeFeeDenominator, ownerWithdrawFeeNumerator, ownerWithdrawFeeDenominator, curveType, poolRegistry, poolNonce);
            transaction.add(instruction);
            yield sendAndConfirmTransaction('createAccount and InitializeSwap', connection, transaction, payer);
            return tokenSwap;
        });
    }
    /**
     * Swap token A for token B
     *
     * @param userSource User's source token account
     * @param poolSource Pool's source token account
     * @param poolDestination Pool's destination token account
     * @param userDestination User's destination token account
     * @param userTransferAuthority Account delegated to transfer user's tokens
     * @param amountIn Amount to transfer from source account
     * @param minimumAmountOut Minimum amount of tokens the user will receive
     */
    swap(userSource, poolSource, poolDestination, userDestination, userTransferAuthority, amountIn, minimumAmountOut) {
        return __awaiter(this, void 0, void 0, function* () {
            return yield sendAndConfirmTransaction('swap', this.connection, new Transaction().add(TokenSwap.swapInstruction(this.tokenSwap, this.authority, userTransferAuthority.publicKey, userSource, poolSource, poolDestination, userDestination, this.poolToken, this.feeAccount, this.swapProgramId, this.tokenProgramId, amountIn, minimumAmountOut)), this.payer, userTransferAuthority);
        });
    }
    static swapInstruction(tokenSwap, authority, userTransferAuthority, userSource, poolSource, poolDestination, userDestination, poolMint, feeAccount, swapProgramId, tokenProgramId, amountIn, minimumAmountOut) {
        const dataLayout = BufferLayout.struct([
            BufferLayout.u8('instruction'),
            Layout.uint64('amountIn'),
            Layout.uint64('minimumAmountOut'),
        ]);
        const data = Buffer.alloc(dataLayout.span);
        dataLayout.encode({
            instruction: 1,
            amountIn: new Numberu64(amountIn).toBuffer(),
            minimumAmountOut: new Numberu64(minimumAmountOut).toBuffer(),
        }, data);
        const keys = [
            { pubkey: tokenSwap, isSigner: false, isWritable: false },
            { pubkey: authority, isSigner: false, isWritable: false },
            { pubkey: userTransferAuthority, isSigner: true, isWritable: false },
            { pubkey: userSource, isSigner: false, isWritable: true },
            { pubkey: poolSource, isSigner: false, isWritable: true },
            { pubkey: poolDestination, isSigner: false, isWritable: true },
            { pubkey: userDestination, isSigner: false, isWritable: true },
            { pubkey: poolMint, isSigner: false, isWritable: true },
            { pubkey: feeAccount, isSigner: false, isWritable: true },
            { pubkey: tokenProgramId, isSigner: false, isWritable: false },
        ];
        return new TransactionInstruction({
            keys,
            programId: swapProgramId,
            data,
        });
    }
    /**
     * Deposit tokens into the pool
     * @param userAccountA User account for token A
     * @param userAccountB User account for token B
     * @param poolAccount User account for pool token
     * @param userTransferAuthority Account delegated to transfer user's tokens
     * @param poolTokenAmount Amount of pool tokens to mint
     * @param maximumTokenA The maximum amount of token A to deposit
     * @param maximumTokenB The maximum amount of token B to deposit
     */
    depositAllTokenTypes(userAccountA, userAccountB, poolAccount, userTransferAuthority, poolTokenAmount, maximumTokenA, maximumTokenB) {
        return __awaiter(this, void 0, void 0, function* () {
            return yield sendAndConfirmTransaction('depositAllTokenTypes', this.connection, new Transaction().add(TokenSwap.depositAllTokenTypesInstruction(this.tokenSwap, this.authority, userTransferAuthority.publicKey, userAccountA, userAccountB, this.tokenAccountA, this.tokenAccountB, this.poolToken, poolAccount, this.swapProgramId, this.tokenProgramId, poolTokenAmount, maximumTokenA, maximumTokenB)), this.payer, userTransferAuthority);
        });
    }
    static depositAllTokenTypesInstruction(tokenSwap, authority, userTransferAuthority, sourceA, sourceB, intoA, intoB, poolToken, poolAccount, swapProgramId, tokenProgramId, poolTokenAmount, maximumTokenA, maximumTokenB) {
        const dataLayout = BufferLayout.struct([
            BufferLayout.u8('instruction'),
            Layout.uint64('poolTokenAmount'),
            Layout.uint64('maximumTokenA'),
            Layout.uint64('maximumTokenB'),
        ]);
        const data = Buffer.alloc(dataLayout.span);
        dataLayout.encode({
            instruction: 2,
            poolTokenAmount: new Numberu64(poolTokenAmount).toBuffer(),
            maximumTokenA: new Numberu64(maximumTokenA).toBuffer(),
            maximumTokenB: new Numberu64(maximumTokenB).toBuffer(),
        }, data);
        const keys = [
            { pubkey: tokenSwap, isSigner: false, isWritable: false },
            { pubkey: authority, isSigner: false, isWritable: false },
            { pubkey: userTransferAuthority, isSigner: true, isWritable: false },
            { pubkey: sourceA, isSigner: false, isWritable: true },
            { pubkey: sourceB, isSigner: false, isWritable: true },
            { pubkey: intoA, isSigner: false, isWritable: true },
            { pubkey: intoB, isSigner: false, isWritable: true },
            { pubkey: poolToken, isSigner: false, isWritable: true },
            { pubkey: poolAccount, isSigner: false, isWritable: true },
            { pubkey: tokenProgramId, isSigner: false, isWritable: false },
        ];
        return new TransactionInstruction({
            keys,
            programId: swapProgramId,
            data,
        });
    }
    /**
     * Withdraw tokens from the pool
     *
     * @param userAccountA User account for token A
     * @param userAccountB User account for token B
     * @param poolAccount User account for pool token
     * @param userTransferAuthority Account delegated to transfer user's tokens
     * @param poolTokenAmount Amount of pool tokens to burn
     * @param minimumTokenA The minimum amount of token A to withdraw
     * @param minimumTokenB The minimum amount of token B to withdraw
     */
    withdrawAllTokenTypes(userAccountA, userAccountB, poolAccount, userTransferAuthority, poolTokenAmount, minimumTokenA, minimumTokenB) {
        return __awaiter(this, void 0, void 0, function* () {
            return yield sendAndConfirmTransaction('withdraw', this.connection, new Transaction().add(TokenSwap.withdrawAllTokenTypesInstruction(this.tokenSwap, this.authority, userTransferAuthority.publicKey, this.poolToken, this.feeAccount, poolAccount, this.tokenAccountA, this.tokenAccountB, userAccountA, userAccountB, this.swapProgramId, this.tokenProgramId, poolTokenAmount, minimumTokenA, minimumTokenB)), this.payer, userTransferAuthority);
        });
    }
    static withdrawAllTokenTypesInstruction(tokenSwap, authority, userTransferAuthority, poolMint, feeAccount, sourcePoolAccount, fromA, fromB, userAccountA, userAccountB, swapProgramId, tokenProgramId, poolTokenAmount, minimumTokenA, minimumTokenB) {
        const dataLayout = BufferLayout.struct([
            BufferLayout.u8('instruction'),
            Layout.uint64('poolTokenAmount'),
            Layout.uint64('minimumTokenA'),
            Layout.uint64('minimumTokenB'),
        ]);
        const data = Buffer.alloc(dataLayout.span);
        dataLayout.encode({
            instruction: 3,
            poolTokenAmount: new Numberu64(poolTokenAmount).toBuffer(),
            minimumTokenA: new Numberu64(minimumTokenA).toBuffer(),
            minimumTokenB: new Numberu64(minimumTokenB).toBuffer(),
        }, data);
        const keys = [
            { pubkey: tokenSwap, isSigner: false, isWritable: false },
            { pubkey: authority, isSigner: false, isWritable: false },
            { pubkey: userTransferAuthority, isSigner: true, isWritable: false },
            { pubkey: poolMint, isSigner: false, isWritable: true },
            { pubkey: sourcePoolAccount, isSigner: false, isWritable: true },
            { pubkey: fromA, isSigner: false, isWritable: true },
            { pubkey: fromB, isSigner: false, isWritable: true },
            { pubkey: userAccountA, isSigner: false, isWritable: true },
            { pubkey: userAccountB, isSigner: false, isWritable: true },
            { pubkey: feeAccount, isSigner: false, isWritable: true },
            { pubkey: tokenProgramId, isSigner: false, isWritable: false },
        ];
        return new TransactionInstruction({
            keys,
            programId: swapProgramId,
            data,
        });
    }
    /**
     * Deposit one side of tokens into the pool
     * @param userAccount User account to deposit token A or B
     * @param poolAccount User account to receive pool tokens
     * @param userTransferAuthority Account delegated to transfer user's tokens
     * @param sourceTokenAmount The amount of token A or B to deposit
     * @param minimumPoolTokenAmount Minimum amount of pool tokens to mint
     */
    depositSingleTokenTypeExactAmountIn(userAccount, poolAccount, userTransferAuthority, sourceTokenAmount, minimumPoolTokenAmount) {
        return __awaiter(this, void 0, void 0, function* () {
            return yield sendAndConfirmTransaction('depositSingleTokenTypeExactAmountIn', this.connection, new Transaction().add(TokenSwap.depositSingleTokenTypeExactAmountInInstruction(this.tokenSwap, this.authority, userTransferAuthority.publicKey, userAccount, this.tokenAccountA, this.tokenAccountB, this.poolToken, poolAccount, this.swapProgramId, this.tokenProgramId, sourceTokenAmount, minimumPoolTokenAmount)), this.payer, userTransferAuthority);
        });
    }
    static depositSingleTokenTypeExactAmountInInstruction(tokenSwap, authority, userTransferAuthority, source, intoA, intoB, poolToken, poolAccount, swapProgramId, tokenProgramId, sourceTokenAmount, minimumPoolTokenAmount) {
        const dataLayout = BufferLayout.struct([
            BufferLayout.u8('instruction'),
            Layout.uint64('sourceTokenAmount'),
            Layout.uint64('minimumPoolTokenAmount'),
        ]);
        const data = Buffer.alloc(dataLayout.span);
        dataLayout.encode({
            instruction: 4,
            sourceTokenAmount: new Numberu64(sourceTokenAmount).toBuffer(),
            minimumPoolTokenAmount: new Numberu64(minimumPoolTokenAmount).toBuffer(),
        }, data);
        const keys = [
            { pubkey: tokenSwap, isSigner: false, isWritable: false },
            { pubkey: authority, isSigner: false, isWritable: false },
            { pubkey: userTransferAuthority, isSigner: true, isWritable: false },
            { pubkey: source, isSigner: false, isWritable: true },
            { pubkey: intoA, isSigner: false, isWritable: true },
            { pubkey: intoB, isSigner: false, isWritable: true },
            { pubkey: poolToken, isSigner: false, isWritable: true },
            { pubkey: poolAccount, isSigner: false, isWritable: true },
            { pubkey: tokenProgramId, isSigner: false, isWritable: false },
        ];
        return new TransactionInstruction({
            keys,
            programId: swapProgramId,
            data,
        });
    }
    /**
     * Withdraw tokens from the pool
     *
     * @param userAccount User account to receive token A or B
     * @param poolAccount User account to burn pool token
     * @param userTransferAuthority Account delegated to transfer user's tokens
     * @param destinationTokenAmount The amount of token A or B to withdraw
     * @param maximumPoolTokenAmount Maximum amount of pool tokens to burn
     */
    withdrawSingleTokenTypeExactAmountOut(userAccount, poolAccount, userTransferAuthority, destinationTokenAmount, maximumPoolTokenAmount) {
        return __awaiter(this, void 0, void 0, function* () {
            return yield sendAndConfirmTransaction('withdrawSingleTokenTypeExactAmountOut', this.connection, new Transaction().add(TokenSwap.withdrawSingleTokenTypeExactAmountOutInstruction(this.tokenSwap, this.authority, userTransferAuthority.publicKey, this.poolToken, this.feeAccount, poolAccount, this.tokenAccountA, this.tokenAccountB, userAccount, this.swapProgramId, this.tokenProgramId, destinationTokenAmount, maximumPoolTokenAmount)), this.payer, userTransferAuthority);
        });
    }
    static withdrawSingleTokenTypeExactAmountOutInstruction(tokenSwap, authority, userTransferAuthority, poolMint, feeAccount, sourcePoolAccount, fromA, fromB, userAccount, swapProgramId, tokenProgramId, destinationTokenAmount, maximumPoolTokenAmount) {
        const dataLayout = BufferLayout.struct([
            BufferLayout.u8('instruction'),
            Layout.uint64('destinationTokenAmount'),
            Layout.uint64('maximumPoolTokenAmount'),
        ]);
        const data = Buffer.alloc(dataLayout.span);
        dataLayout.encode({
            instruction: 5,
            destinationTokenAmount: new Numberu64(destinationTokenAmount).toBuffer(),
            maximumPoolTokenAmount: new Numberu64(maximumPoolTokenAmount).toBuffer(),
        }, data);
        const keys = [
            { pubkey: tokenSwap, isSigner: false, isWritable: false },
            { pubkey: authority, isSigner: false, isWritable: false },
            { pubkey: userTransferAuthority, isSigner: true, isWritable: false },
            { pubkey: poolMint, isSigner: false, isWritable: true },
            { pubkey: sourcePoolAccount, isSigner: false, isWritable: true },
            { pubkey: fromA, isSigner: false, isWritable: true },
            { pubkey: fromB, isSigner: false, isWritable: true },
            { pubkey: userAccount, isSigner: false, isWritable: true },
            { pubkey: feeAccount, isSigner: false, isWritable: true },
            { pubkey: tokenProgramId, isSigner: false, isWritable: false },
        ];
        return new TransactionInstruction({
            keys,
            programId: swapProgramId,
            data,
        });
    }
}
