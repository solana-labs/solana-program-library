import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { AccountMeta, Connection, Signer } from '@solana/web3.js';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';
import {
    createInitializeMintInstruction,
    getMint,
    getMintLen,
    ExtensionType,
    createInitializeTransferHookInstruction,
    getTransferHook,
    updateTransferHook,
    AuthorityType,
    setAuthority,
    createAssociatedTokenAccountInstruction,
    getAssociatedTokenAddressSync,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createMintToCheckedInstruction,
    getExtraAccountMetaAddress,
    ExtraAccountMetaListLayout,
    ExtraAccountMetaLayout,
    transferCheckedWithTransferHook,
    createAssociatedTokenAccountIdempotent,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection, TRANSFER_HOOK_TEST_PROGRAM_ID } from '../common';
import { createHash } from 'crypto';

const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.TransferHook];
describe('transferHook', () => {
    let connection: Connection;
    let payer: Signer;
    let payerAta: PublicKey;
    let destinationAuthority: PublicKey;
    let destinationAta: PublicKey;
    let transferHookAuthority: Keypair;
    let pdaExtraAccountMeta: PublicKey;
    let mint: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        destinationAuthority = Keypair.generate().publicKey;
        transferHookAuthority = Keypair.generate();
    });
    beforeEach(async () => {
        const mintKeypair = Keypair.generate();
        mint = mintKeypair.publicKey;
        pdaExtraAccountMeta = getExtraAccountMetaAddress(mint, TRANSFER_HOOK_TEST_PROGRAM_ID);
        payerAta = getAssociatedTokenAddressSync(
            mint,
            payer.publicKey,
            false,
            TEST_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );
        destinationAta = getAssociatedTokenAddressSync(
            mint,
            destinationAuthority,
            false,
            TEST_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );
        const mintLen = getMintLen(EXTENSIONS);
        const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);
        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mint,
                space: mintLen,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeTransferHookInstruction(
                mint,
                transferHookAuthority.publicKey,
                TRANSFER_HOOK_TEST_PROGRAM_ID,
                TEST_PROGRAM_ID
            ),
            createInitializeMintInstruction(mint, TEST_TOKEN_DECIMALS, payer.publicKey, null, TEST_PROGRAM_ID)
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair], undefined);
    });
    it('is initialized', async () => {
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const transferHook = getTransferHook(mintInfo);
        expect(transferHook).to.not.be.null;
        if (transferHook !== null) {
            expect(transferHook.authority).to.eql(transferHookAuthority.publicKey);
            expect(transferHook.programId).to.eql(TRANSFER_HOOK_TEST_PROGRAM_ID);
        }
    });
    it('can be updated', async () => {
        const newTransferHookProgramId = Keypair.generate().publicKey;
        await updateTransferHook(
            connection,
            payer,
            mint,
            newTransferHookProgramId,
            transferHookAuthority,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const transferHook = getTransferHook(mintInfo);
        expect(transferHook).to.not.be.null;
        if (transferHook !== null) {
            expect(transferHook.authority).to.eql(transferHookAuthority.publicKey);
            expect(transferHook.programId).to.eql(newTransferHookProgramId);
        }
    });
    it('authority', async () => {
        await setAuthority(
            connection,
            payer,
            mint,
            transferHookAuthority,
            AuthorityType.TransferHookProgramId,
            null,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const transferHook = getTransferHook(mintInfo);
        expect(transferHook).to.not.be.null;
        if (transferHook !== null) {
            expect(transferHook.authority).to.eql(PublicKey.default);
        }
    });
    it('transferChecked', async () => {
        const extraAccount = Keypair.generate().publicKey;
        const keys: AccountMeta[] = [
            { pubkey: pdaExtraAccountMeta, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
            { pubkey: payer.publicKey, isSigner: true, isWritable: false },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ];

        const data = Buffer.alloc(8 + 4 + ExtraAccountMetaLayout.span);
        const discriminator = createHash('sha256')
            .update('spl-transfer-hook-interface:initialize-extra-account-metas')
            .digest()
            .subarray(0, 8);
        discriminator.copy(data);
        ExtraAccountMetaListLayout.encode(
            {
                count: 1,
                extraAccounts: [
                    {
                        discriminator: 0,
                        addressConfig: extraAccount.toBuffer(),
                        isSigner: false,
                        isWritable: false,
                    },
                ],
            },
            data,
            8
        );

        const initExtraAccountMetaInstruction = new TransactionInstruction({
            keys,
            data,
            programId: TRANSFER_HOOK_TEST_PROGRAM_ID,
        });

        const setupTransaction = new Transaction().add(
            initExtraAccountMetaInstruction,
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: pdaExtraAccountMeta,
                lamports: 10000000,
            }),
            createAssociatedTokenAccountInstruction(
                payer.publicKey,
                payerAta,
                payer.publicKey,
                mint,
                TEST_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID
            ),
            createMintToCheckedInstruction(
                mint,
                payerAta,
                payer.publicKey,
                5 * 10 ** TEST_TOKEN_DECIMALS,
                TEST_TOKEN_DECIMALS,
                [],
                TEST_PROGRAM_ID
            )
        );

        await sendAndConfirmTransaction(connection, setupTransaction, [payer]);

        await createAssociatedTokenAccountIdempotent(
            connection,
            payer,
            mint,
            destinationAuthority,
            undefined,
            TEST_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        await transferCheckedWithTransferHook(
            connection,
            payer,
            payerAta,
            mint,
            destinationAta,
            payer,
            BigInt(10 ** TEST_TOKEN_DECIMALS),
            TEST_TOKEN_DECIMALS,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
    });
});
