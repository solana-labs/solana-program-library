import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';
import {
    AccountState,
    createAccount,
    createInitializeMintInstruction,
    createInitializeDefaultAccountStateInstruction,
    getAccount,
    getDefaultAccountState,
    getMint,
    getMintLen,
    updateDefaultAccountState,
    ExtensionType,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_STATE = AccountState.Frozen;
const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.DefaultAccountState];
describe('defaultAccountState', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let freezeAuthority: Keypair;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
        freezeAuthority = Keypair.generate();
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
            createInitializeDefaultAccountStateInstruction(mint, TEST_STATE, TEST_PROGRAM_ID),
            createInitializeMintInstruction(
                mint,
                TEST_TOKEN_DECIMALS,
                mintAuthority.publicKey,
                freezeAuthority.publicKey,
                TEST_PROGRAM_ID
            )
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair], undefined);
    });
    it('defaults to frozen', async () => {
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const defaultAccountState = getDefaultAccountState(mintInfo);
        expect(defaultAccountState).to.not.be.null;
        if (defaultAccountState !== null) {
            expect(defaultAccountState.state).to.eql(TEST_STATE);
        }
        const owner = Keypair.generate();
        const account = await createAccount(
            connection,
            payer,
            mint,
            owner.publicKey,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );
        const accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
        expect(accountInfo.isFrozen).to.be.true;
        expect(accountInfo.isInitialized).to.be.true;
    });
    it('defaults to initialized after update', async () => {
        await updateDefaultAccountState(
            connection,
            payer,
            mint,
            AccountState.Initialized,
            freezeAuthority,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const owner = Keypair.generate();
        const account = await createAccount(
            connection,
            payer,
            mint,
            owner.publicKey,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );
        const accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
        expect(accountInfo.isFrozen).to.be.false;
        expect(accountInfo.isInitialized).to.be.true;
    });
});
