import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { Keypair } from '@solana/web3.js';
import { createMint, createAccount, closeAccount, mintTo } from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
describe('close', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let freezeAuthority: Keypair;
    let owner: Keypair;
    let account: PublicKey;
    let destination: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
        freezeAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();
        mint = await createMint(
            connection,
            payer,
            mintAuthority.publicKey,
            freezeAuthority.publicKey,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
            undefined,
            TEST_PROGRAM_ID
        );
    });
    beforeEach(async () => {
        owner = Keypair.generate();
        destination = Keypair.generate().publicKey;
        account = await createAccount(connection, payer, mint, owner.publicKey, undefined, undefined, TEST_PROGRAM_ID);
    });
    it('failsWithNonZeroAmount', async () => {
        const amount = BigInt(1000);
        await mintTo(connection, payer, mint, account, mintAuthority, amount, [], undefined, TEST_PROGRAM_ID);
        expect(closeAccount(connection, payer, account, destination, owner, [], undefined, TEST_PROGRAM_ID)).to.be
            .rejected;
    });
    it('works', async () => {
        const accountInfo = await connection.getAccountInfo(account);
        let tokenRentExemptAmount;
        expect(accountInfo).to.not.be.null;
        if (accountInfo !== null) {
            tokenRentExemptAmount = accountInfo.lamports;
        }

        await closeAccount(connection, payer, account, destination, owner, [], undefined, TEST_PROGRAM_ID);

        const closedInfo = await connection.getAccountInfo(account);
        expect(closedInfo).to.be.null;

        const destinationInfo = await connection.getAccountInfo(destination);
        expect(destinationInfo).to.not.be.null;
        if (destinationInfo !== null) {
            expect(destinationInfo.lamports).to.eql(tokenRentExemptAmount);
        }
    });
});
