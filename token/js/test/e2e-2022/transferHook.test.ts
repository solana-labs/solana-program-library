import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, Signer } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
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
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.TransferHook];
describe('transferHook', () => {
    let connection: Connection;
    let payer: Signer;
    let transferHookAuthority: Keypair;
    let mint: PublicKey;
    let transferHookProgramId: PublicKey;
    let newTransferHookProgramId: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        transferHookAuthority = Keypair.generate();
        transferHookProgramId = Keypair.generate().publicKey;
        newTransferHookProgramId = Keypair.generate().publicKey;
    });
    beforeEach(async () => {
        const mintKeypair = Keypair.generate();
        mint = mintKeypair.publicKey;
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
                transferHookProgramId,
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
            expect(transferHook.programId).to.eql(transferHookProgramId);
        }
    });
    it('can be updated', async () => {
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
});
