import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';
import {
    createInitializeMintInstruction,
    getMint,
    getMintLen,
    ExtensionType,
    createInitializeTransferHookInstruction,
    getTransferHook,
    updateTransferHook,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.TransferHook];
describe('transferHook', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let programId: PublicKey;
    let programId2: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        programId = Keypair.generate().publicKey;
        programId2 = Keypair.generate().publicKey;
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
            createInitializeTransferHookInstruction(mint, payer.publicKey, programId, TEST_PROGRAM_ID),
            createInitializeMintInstruction(mint, TEST_TOKEN_DECIMALS, payer.publicKey, null, TEST_PROGRAM_ID)
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair], undefined);
    });
    it('is initialized', async () => {
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const transferHook = getTransferHook(mintInfo);
        expect(transferHook).to.not.be.null;
        if (transferHook !== null) {
            expect(transferHook.authority.toString()).to.eql(payer.publicKey.toString());
            expect(transferHook.programId.toString()).to.eql(programId.toString());
        }
    });
    it('can be updated', async () => {
        await updateTransferHook(connection, payer, mint, programId2, payer.publicKey, [], undefined, TEST_PROGRAM_ID);
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const transferHook = getTransferHook(mintInfo);
        expect(transferHook).to.not.be.null;
        if (transferHook !== null) {
            expect(transferHook.authority.toString()).to.eql(payer.publicKey.toString());
            expect(transferHook.programId.toString()).to.eql(programId2.toString());
        }
    });
});
