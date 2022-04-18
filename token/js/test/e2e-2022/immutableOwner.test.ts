import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import {
    Connection,
    Keypair,
    PublicKey,
    Signer,
    SystemProgram,
    Transaction,
    sendAndConfirmTransaction,
} from '@solana/web3.js';

import {
    AuthorityType,
    setAuthority,
    ExtensionType,
    createInitializeImmutableOwnerInstruction,
    getAccountLen,
} from '../../src';

import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const EXTENSIONS = [ExtensionType.ImmutableOwner];
describe('immutableOwner', () => {
    let connection: Connection;
    let payer: Signer;
    let owner: Keypair;
    let account: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
    });
    beforeEach(async () => {
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
            createInitializeImmutableOwnerInstruction(account, TEST_PROGRAM_ID)
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
