import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { Keypair, Transaction, sendAndConfirmTransaction } from '@solana/web3.js';

import { ExtensionType, createAccount, createMint, createReallocateInstruction, getAccountLen } from '../../src';

import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';
const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.ImmutableOwner];
describe('reallocate', () => {
    let connection: Connection;
    let payer: Signer;
    let owner: Keypair;
    let account: PublicKey;
    let mint: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
    });
    beforeEach(async () => {
        const mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();
        mint = await createMint(
            connection,
            payer,
            mintAuthority.publicKey,
            mintAuthority.publicKey,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
            undefined,
            TEST_PROGRAM_ID
        );
        owner = Keypair.generate();
        account = await createAccount(connection, payer, mint, owner.publicKey, undefined, undefined, TEST_PROGRAM_ID);
    });
    it('works', async () => {
        const transaction = new Transaction().add(
            createReallocateInstruction(
                account,
                payer.publicKey,
                EXTENSIONS,
                owner.publicKey,
                undefined,
                TEST_PROGRAM_ID
            )
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, owner], undefined);
        const info = await connection.getAccountInfo(account);
        expect(info).to.not.be.null;
        if (info !== null) {
            const expectedAccountLen = getAccountLen(EXTENSIONS);
            expect(info.data.length).to.eql(expectedAccountLen);
        }
    });
});
