import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';
import {
    createAccount,
    createMint,
    createEnableCpiGuardInstruction,
    createDisableCpiGuardInstruction,
    createInitializeAccountInstruction,
    getAccount,
    getCpiGuard,
    enableCpiGuard,
    disableCpiGuard,
    getAccountLen,
    ExtensionType,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
const TRANSFER_AMOUNT = 1_000;
const EXTENSIONS = [ExtensionType.CpiGuard];
describe('cpiGuard', () => {
    let connection: Connection;
    let payer: Signer;
    let owner: Keypair;
    let account: PublicKey;

    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        owner = Keypair.generate();
    });

    beforeEach(async () => {
        const mintKeypair = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const accountKeypair = Keypair.generate();
        account = accountKeypair.publicKey;
        const accountLen = getAccountLen(EXTENSIONS);
        const lamports = await connection.getMinimumBalanceForRentExemption(accountLen);

        const mint = await createMint(
            connection,
            payer,
            mintAuthority.publicKey,
            mintAuthority.publicKey,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
            undefined,
            TEST_PROGRAM_ID
        );

        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: account,
                space: accountLen,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeAccountInstruction(account, mint, owner.publicKey, TEST_PROGRAM_ID)
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, accountKeypair], undefined);
    });

    it('enable/disable via instruction', async () => {
        let accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
        let cpiGuard = getCpiGuard(accountInfo);

        expect(cpiGuard).to.be.null;

        let transaction = new Transaction().add(
            createEnableCpiGuardInstruction(account, owner.publicKey, [], TEST_PROGRAM_ID)
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, owner], undefined);

        accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
        cpiGuard = getCpiGuard(accountInfo);

        expect(cpiGuard).to.not.be.null;
        if (cpiGuard !== null) {
            expect(cpiGuard.lockCpi).to.be.true;
        }

        transaction = new Transaction().add(
            createDisableCpiGuardInstruction(account, owner.publicKey, [], TEST_PROGRAM_ID)
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, owner], undefined);

        accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
        cpiGuard = getCpiGuard(accountInfo);

        expect(cpiGuard).to.not.be.null;
        if (cpiGuard !== null) {
            expect(cpiGuard.lockCpi).to.be.false;
        }
    });

    it('enable/disable via command', async () => {
        let accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
        let cpiGuard = getCpiGuard(accountInfo);

        expect(cpiGuard).to.be.null;

        await enableCpiGuard(connection, payer, account, owner, [], undefined, TEST_PROGRAM_ID);

        accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
        cpiGuard = getCpiGuard(accountInfo);

        expect(cpiGuard).to.not.be.null;
        if (cpiGuard !== null) {
            expect(cpiGuard.lockCpi).to.be.true;
        }

        await disableCpiGuard(connection, payer, account, owner, [], undefined, TEST_PROGRAM_ID);

        accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
        cpiGuard = getCpiGuard(accountInfo);

        expect(cpiGuard).to.not.be.null;
        if (cpiGuard !== null) {
            expect(cpiGuard.lockCpi).to.be.false;
        }
    });
});
