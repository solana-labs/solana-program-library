import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, Signer } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';
import {
    createAccount,
    createInitializeMintInstruction,
    mintTo,
    getMintLen,
    ExtensionType,
    createInitializePermanentDelegateInstruction,
    burn,
    transferChecked,
    AuthorityType,
    getMint,
    setAuthority,
    getPermanentDelegate,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 0;
const EXTENSIONS = [ExtensionType.PermanentDelegate];
describe('permanentDelegate', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let permanentDelegate: Keypair;
    let account: PublicKey;
    let destination: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
        permanentDelegate = Keypair.generate();
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
            createInitializePermanentDelegateInstruction(mint, permanentDelegate.publicKey, TEST_PROGRAM_ID),
            createInitializeMintInstruction(mint, TEST_TOKEN_DECIMALS, mintAuthority.publicKey, null, TEST_PROGRAM_ID)
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair], undefined);
    });
    it('burn tokens ', async () => {
        const owner = Keypair.generate();
        account = await createAccount(connection, payer, mint, owner.publicKey, undefined, undefined, TEST_PROGRAM_ID);
        await mintTo(connection, payer, mint, account, mintAuthority, 5, [], undefined, TEST_PROGRAM_ID);
        await burn(connection, payer, account, mint, permanentDelegate, 2, undefined, undefined, TEST_PROGRAM_ID);
        const info = await connection.getTokenAccountBalance(account);
        expect(info).to.not.be.null;
        if (info !== null) {
            expect(info.value.uiAmount).to.eql(3);
        }
    });
    it('transfer tokens', async () => {
        const owner1 = Keypair.generate();
        const owner2 = Keypair.generate();
        destination = await createAccount(
            connection,
            payer,
            mint,
            owner2.publicKey,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );
        account = await createAccount(connection, payer, mint, owner1.publicKey, undefined, undefined, TEST_PROGRAM_ID);
        await mintTo(connection, payer, mint, account, mintAuthority, 5, [], undefined, TEST_PROGRAM_ID);
        await transferChecked(
            connection,
            payer,
            account,
            mint,
            destination,
            permanentDelegate,
            2,
            0,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );
        const source_info = await connection.getTokenAccountBalance(account);
        const destination_info = await connection.getTokenAccountBalance(destination);
        expect(source_info).to.not.be.null;
        expect(destination_info).to.not.be.null;
        if (source_info !== null) {
            expect(source_info.value.uiAmount).to.eql(3);
        }
        if (destination_info !== null) {
            expect(destination_info.value.uiAmount).to.eql(2);
        }
    });
    it('authority', async () => {
        await setAuthority(
            connection,
            payer,
            mint,
            permanentDelegate,
            AuthorityType.PermanentDelegate,
            null,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const permanentDelegateConfig = getPermanentDelegate(mintInfo);
        expect(permanentDelegateConfig).to.not.be.null;
        if (permanentDelegateConfig !== null) {
            expect(permanentDelegateConfig.delegate).to.eql(PublicKey.default);
        }
    });
});
