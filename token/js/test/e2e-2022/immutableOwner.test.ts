import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { Keypair, SystemProgram, Transaction, sendAndConfirmTransaction } from '@solana/web3.js';

import {
    AuthorityType,
    setAuthority,
    ExtensionType,
    createInitializeImmutableOwnerInstruction,
    createInitializeAccountInstruction,
    createMint,
    getAccountLen,
} from '../../src';

import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';
const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.ImmutableOwner];
describe('immutableOwner', () => {
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
        const accountLen = getAccountLen(EXTENSIONS);
        const lamports = await connection.getMinimumBalanceForRentExemption(accountLen);
        const accountKeypair = Keypair.generate();
        account = accountKeypair.publicKey;
        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: account,
                space: accountLen,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeImmutableOwnerInstruction(account, TEST_PROGRAM_ID),
            createInitializeAccountInstruction(account, mint, owner.publicKey, TEST_PROGRAM_ID)
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, accountKeypair], undefined);
    });
    it('AccountOwner', async () => {
        const newOwner = Keypair.generate();
        expect(
            setAuthority(
                connection,
                payer,
                account,
                newOwner,
                AuthorityType.AccountOwner,
                owner.publicKey,
                [],
                undefined,
                TEST_PROGRAM_ID
            )
        ).to.be.rejected;
    });
});
