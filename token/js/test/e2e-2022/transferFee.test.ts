import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, Signer } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { Keypair, SystemProgram, Transaction, sendAndConfirmTransaction } from '@solana/web3.js';

import {
    ExtensionType,
    createInitializeMintInstruction,
    getTransferFeeAmount,
    getTransferFeeConfig,
    mintTo,
    transferChecked,
    createAccount,
    getAccount,
    getMint,
    getMintLen,
    setAuthority,
    AuthorityType,
} from '../../src';

import {
    createInitializeTransferFeeConfigInstruction,
    harvestWithheldTokensToMint,
    transferCheckedWithFee,
    withdrawWithheldTokensFromAccounts,
    withdrawWithheldTokensFromMint,
} from '../../src/extensions/transferFee/index';

import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';
const TEST_TOKEN_DECIMALS = 2;
const MINT_EXTENSIONS = [ExtensionType.TransferFeeConfig];
const MINT_AMOUNT = BigInt(1_000_000_000);
const TRANSFER_AMOUNT = BigInt(1_000_000);
const FEE_BASIS_POINTS = 100;
const MAX_FEE = BigInt(100_000);
const FEE = (TRANSFER_AMOUNT * BigInt(FEE_BASIS_POINTS)) / BigInt(10_000);
describe('transferFee', () => {
    let connection: Connection;
    let payer: Signer;
    let owner: Keypair;
    let sourceAccount: PublicKey;
    let destinationAccount: PublicKey;
    let mint: PublicKey;
    let transferFeeConfigAuthority: Keypair;
    let withdrawWithheldAuthority: Keypair;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        transferFeeConfigAuthority = Keypair.generate();
        withdrawWithheldAuthority = Keypair.generate();
    });
    beforeEach(async () => {
        const mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();
        mint = mintKeypair.publicKey;
        const mintLen = getMintLen(MINT_EXTENSIONS);
        const mintLamports = await connection.getMinimumBalanceForRentExemption(mintLen);
        const mintTransaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mint,
                space: mintLen,
                lamports: mintLamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeTransferFeeConfigInstruction(
                mint,
                transferFeeConfigAuthority.publicKey,
                withdrawWithheldAuthority.publicKey,
                FEE_BASIS_POINTS,
                MAX_FEE,
                TEST_PROGRAM_ID
            ),
            createInitializeMintInstruction(mint, TEST_TOKEN_DECIMALS, mintAuthority.publicKey, null, TEST_PROGRAM_ID)
        );
        await sendAndConfirmTransaction(connection, mintTransaction, [payer, mintKeypair], undefined);

        owner = Keypair.generate();
        sourceAccount = await createAccount(
            connection,
            payer,
            mint,
            owner.publicKey,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );
        await mintTo(
            connection,
            payer,
            mint,
            sourceAccount,
            mintAuthority,
            MINT_AMOUNT,
            [],
            undefined,
            TEST_PROGRAM_ID
        );

        const accountKeypair = Keypair.generate();
        destinationAccount = await createAccount(
            connection,
            payer,
            mint,
            owner.publicKey,
            accountKeypair,
            undefined,
            TEST_PROGRAM_ID
        );

        await transferChecked(
            connection,
            payer,
            sourceAccount,
            mint,
            destinationAccount,
            owner,
            TRANSFER_AMOUNT,
            TEST_TOKEN_DECIMALS,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
    });
    it('initializes', async () => {
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const transferFeeConfig = getTransferFeeConfig(mintInfo);
        expect(transferFeeConfig).to.not.be.null;
        if (transferFeeConfig !== null) {
            expect(transferFeeConfig.transferFeeConfigAuthority).to.eql(transferFeeConfigAuthority.publicKey);
            expect(transferFeeConfig.withdrawWithheldAuthority).to.eql(withdrawWithheldAuthority.publicKey);
            expect(transferFeeConfig.olderTransferFee.transferFeeBasisPoints).to.eql(FEE_BASIS_POINTS);
            expect(transferFeeConfig.olderTransferFee.maximumFee).to.eql(MAX_FEE);
            expect(transferFeeConfig.newerTransferFee.transferFeeBasisPoints).to.eql(FEE_BASIS_POINTS);
            expect(transferFeeConfig.newerTransferFee.maximumFee).to.eql(MAX_FEE);
            expect(transferFeeConfig.withheldAmount).to.eql(BigInt(0));
        }

        const accountInfo = await getAccount(connection, destinationAccount, undefined, TEST_PROGRAM_ID);
        const transferFeeAmount = getTransferFeeAmount(accountInfo);
        expect(transferFeeAmount).to.not.be.null;
        if (transferFeeAmount !== null) {
            expect(transferFeeAmount.withheldAmount).to.eql(FEE);
        }
    });
    it('transferCheckedWithFee', async () => {
        await transferCheckedWithFee(
            connection,
            payer,
            sourceAccount,
            mint,
            destinationAccount,
            owner,
            TRANSFER_AMOUNT,
            TEST_TOKEN_DECIMALS,
            FEE,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const accountInfo = await getAccount(connection, destinationAccount, undefined, TEST_PROGRAM_ID);
        const transferFeeAmount = getTransferFeeAmount(accountInfo);
        expect(transferFeeAmount).to.not.be.null;
        if (transferFeeAmount !== null) {
            expect(transferFeeAmount.withheldAmount).to.eql(FEE * BigInt(2));
        }
    });
    it('withdrawWithheldTokensFromAccounts', async () => {
        await withdrawWithheldTokensFromAccounts(
            connection,
            payer,
            mint,
            destinationAccount,
            withdrawWithheldAuthority,
            [],
            [destinationAccount],
            undefined,
            TEST_PROGRAM_ID
        );
        const accountInfo = await getAccount(connection, destinationAccount, undefined, TEST_PROGRAM_ID);
        expect(accountInfo.amount).to.eql(TRANSFER_AMOUNT);
        const transferFeeAmount = getTransferFeeAmount(accountInfo);
        expect(transferFeeAmount).to.not.be.null;
        if (transferFeeAmount !== null) {
            expect(transferFeeAmount.withheldAmount).to.eql(BigInt(0));
        }
    });
    it('harvestWithheldTokensToMint', async () => {
        await harvestWithheldTokensToMint(connection, payer, mint, [destinationAccount], undefined, TEST_PROGRAM_ID);
        const accountInfo = await getAccount(connection, destinationAccount, undefined, TEST_PROGRAM_ID);
        const transferFeeAmount = getTransferFeeAmount(accountInfo);
        expect(transferFeeAmount).to.not.be.null;
        if (transferFeeAmount !== null) {
            expect(transferFeeAmount.withheldAmount).to.eql(BigInt(0));
        }
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const transferFeeConfig = getTransferFeeConfig(mintInfo);
        expect(transferFeeConfig).to.not.be.null;
        if (transferFeeConfig !== null) {
            expect(transferFeeConfig.withheldAmount).to.eql(FEE);
        }
    });
    it('withdrawWithheldTokensFromMint', async () => {
        await harvestWithheldTokensToMint(connection, payer, mint, [destinationAccount], undefined, TEST_PROGRAM_ID);
        await withdrawWithheldTokensFromMint(
            connection,
            payer,
            mint,
            destinationAccount,
            withdrawWithheldAuthority,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const accountInfo = await getAccount(connection, destinationAccount, undefined, TEST_PROGRAM_ID);
        expect(accountInfo.amount).to.eql(TRANSFER_AMOUNT);
        const transferFeeAmount = getTransferFeeAmount(accountInfo);
        expect(transferFeeAmount).to.not.be.null;
        if (transferFeeAmount !== null) {
            expect(transferFeeAmount.withheldAmount).to.eql(BigInt(0));
        }
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const transferFeeConfig = getTransferFeeConfig(mintInfo);
        expect(transferFeeConfig).to.not.be.null;
        if (transferFeeConfig !== null) {
            expect(transferFeeConfig.withheldAmount).to.eql(BigInt(0));
        }
    });
    it('transferFeeConfigAuthority', async () => {
        await setAuthority(
            connection,
            payer,
            mint,
            transferFeeConfigAuthority,
            AuthorityType.TransferFeeConfig,
            null,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const transferFeeConfig = getTransferFeeConfig(mintInfo);
        expect(transferFeeConfig).to.not.be.null;
        if (transferFeeConfig !== null) {
            expect(transferFeeConfig.transferFeeConfigAuthority).to.eql(PublicKey.default);
        }
    });
    it('withdrawWithheldAuthority', async () => {
        await setAuthority(
            connection,
            payer,
            mint,
            withdrawWithheldAuthority,
            AuthorityType.WithheldWithdraw,
            null,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const transferFeeConfig = getTransferFeeConfig(mintInfo);
        expect(transferFeeConfig).to.not.be.null;
        if (transferFeeConfig !== null) {
            expect(transferFeeConfig.withdrawWithheldAuthority).to.eql(PublicKey.default);
        }
    });
});
