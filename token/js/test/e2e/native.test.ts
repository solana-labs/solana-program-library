import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import {
    Connection,
    Keypair,
    PublicKey,
    Signer,
    Transaction,
    SystemProgram,
    sendAndConfirmTransaction,
} from '@solana/web3.js';

import { TOKEN_PROGRAM_ID, closeAccount, getAccount, createWrappedNativeAccount, syncNative } from '../../src';

import { newAccountWithLamports, getConnection } from './common';

describe('native', () => {
    let connection: Connection;
    let payer: Signer;
    let owner: Keypair;
    let account: PublicKey;
    let amount: number;
    before(async () => {
        amount = 1_000_000_000;
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 100_000_000_000);
    });
    beforeEach(async () => {
        owner = Keypair.generate();
        account = await createWrappedNativeAccount(
            connection,
            payer,
            owner.publicKey,
            amount,
            undefined,
            undefined,
            TOKEN_PROGRAM_ID
        );
    });
    it('works', async () => {
        const accountInfo = await getAccount(connection, account, undefined, TOKEN_PROGRAM_ID);
        expect(accountInfo.isNative).to.be.true;
        expect(accountInfo.amount).to.eql(BigInt(amount));
    });
    it('syncNative', async () => {
        let balance = 0;
        const preInfo = await connection.getAccountInfo(account);
        expect(preInfo).to.not.be.null;
        if (preInfo != null) {
            balance = preInfo.lamports;
        }

        // transfer lamports into the native account
        const additionalLamports = 100;
        await sendAndConfirmTransaction(
            connection,
            new Transaction().add(
                SystemProgram.transfer({
                    fromPubkey: payer.publicKey,
                    toPubkey: account,
                    lamports: additionalLamports,
                })
            ),
            [payer]
        );

        // no change in the amount
        const preAccountInfo = await getAccount(connection, account, undefined, TOKEN_PROGRAM_ID);
        expect(preAccountInfo.isNative).to.be.true;
        expect(preAccountInfo.amount).to.eql(BigInt(amount));

        // but change in lamports
        const postInfo = await connection.getAccountInfo(account);
        expect(postInfo).to.not.be.null;
        if (postInfo !== null) {
            expect(postInfo.lamports).to.eql(balance + additionalLamports);
        }

        // sync, amount changes
        await syncNative(connection, payer, account, undefined, TOKEN_PROGRAM_ID);
        const postAccountInfo = await getAccount(connection, account, undefined, TOKEN_PROGRAM_ID);
        expect(postAccountInfo.isNative).to.be.true;
        expect(postAccountInfo.amount).to.eql(BigInt(amount + additionalLamports));
    });
    it('closeAccount', async () => {
        let balance = 0;
        const preInfo = await connection.getAccountInfo(account);
        expect(preInfo).to.not.be.null;
        if (preInfo != null) {
            balance = preInfo.lamports;
        }
        const destination = Keypair.generate().publicKey;
        await closeAccount(connection, payer, account, destination, owner, [], undefined, TOKEN_PROGRAM_ID);
        const nullInfo = await connection.getAccountInfo(account);
        expect(nullInfo).to.be.null;
        const destinationInfo = await connection.getAccountInfo(destination);
        expect(destinationInfo).to.not.be.null;
        if (destinationInfo != null) {
            expect(destinationInfo.lamports).to.eql(balance);
        }
    });
});
