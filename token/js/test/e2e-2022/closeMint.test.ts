import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, Signer } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';
import {
    createAccount,
    createInitializeMintInstruction,
    createInitializeMintCloseAuthorityInstruction,
    closeAccount,
    mintTo,
    getMintLen,
    ExtensionType,
    AuthorityType,
    getMint,
    setAuthority,
    getMintCloseAuthority,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.MintCloseAuthority];
describe('closeMint', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let closeAuthority: Keypair;
    let account: PublicKey;
    let destination: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
        closeAuthority = Keypair.generate();
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
            createInitializeMintCloseAuthorityInstruction(mint, closeAuthority.publicKey, TEST_PROGRAM_ID),
            createInitializeMintInstruction(mint, TEST_TOKEN_DECIMALS, mintAuthority.publicKey, null, TEST_PROGRAM_ID)
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair], undefined);
    });
    it('failsWithNonZeroAmount', async () => {
        const owner = Keypair.generate();
        destination = Keypair.generate().publicKey;
        account = await createAccount(connection, payer, mint, owner.publicKey, undefined, undefined, TEST_PROGRAM_ID);
        const amount = BigInt(1000);
        await mintTo(connection, payer, mint, account, mintAuthority, amount, [], undefined, TEST_PROGRAM_ID);
        expect(closeAccount(connection, payer, mint, destination, closeAuthority, [], undefined, TEST_PROGRAM_ID)).to.be
            .rejected;
    });
    it('works', async () => {
        destination = Keypair.generate().publicKey;
        const accountInfo = await connection.getAccountInfo(mint);
        let rentExemptAmount;
        expect(accountInfo).to.not.be.null;
        if (accountInfo !== null) {
            rentExemptAmount = accountInfo.lamports;
        }

        await closeAccount(connection, payer, mint, destination, closeAuthority, [], undefined, TEST_PROGRAM_ID);

        const closedInfo = await connection.getAccountInfo(mint);
        expect(closedInfo).to.be.null;

        const destinationInfo = await connection.getAccountInfo(destination);
        expect(destinationInfo).to.not.be.null;
        if (destinationInfo !== null) {
            expect(destinationInfo.lamports).to.eql(rentExemptAmount);
        }
    });
    it('authority', async () => {
        await setAuthority(
            connection,
            payer,
            mint,
            closeAuthority,
            AuthorityType.CloseMint,
            null,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const mintCloseAuthority = getMintCloseAuthority(mintInfo);
        expect(mintCloseAuthority).to.not.be.null;
        if (mintCloseAuthority !== null) {
            expect(mintCloseAuthority.closeAuthority).to.eql(PublicKey.default);
        }
    });
});
