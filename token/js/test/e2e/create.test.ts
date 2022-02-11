import { Connection, Keypair, PublicKey, Signer } from '@solana/web3.js';

import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import {
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createMint,
    getMint,
    createAccount,
    getAccount,
    getAssociatedTokenAddress,
} from '../../src';

import { newAccountWithLamports, getConnection } from './common';

const TEST_TOKEN_DECIMALS = 2;
describe('createMint', () => {
    it('works', async () => {
        const connection = await getConnection();
        const payer = await newAccountWithLamports(connection, 1000000000);
        const testMintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();
        const mint = await createMint(
            connection,
            payer,
            testMintAuthority.publicKey,
            testMintAuthority.publicKey,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
            undefined,
            TOKEN_PROGRAM_ID
        );

        const mintInfo = await getMint(connection, mint, undefined, TOKEN_PROGRAM_ID);

        expect(mintInfo.mintAuthority).to.eql(testMintAuthority.publicKey);
        expect(mintInfo.supply).to.eql(BigInt(0));
        expect(mintInfo.decimals).to.eql(TEST_TOKEN_DECIMALS);
        expect(mintInfo.isInitialized).to.be.true;
        expect(mintInfo.freezeAuthority).to.eql(testMintAuthority.publicKey);
    });
});

describe('createAccount', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
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
            TOKEN_PROGRAM_ID
        );
    }),
        it('auxiliary token account', async () => {
            const owner = Keypair.generate();
            const account = await createAccount(
                connection,
                payer,
                mint,
                owner.publicKey,
                Keypair.generate(),
                undefined,
                TOKEN_PROGRAM_ID
            );
            const accountInfo = await getAccount(connection, account, undefined, TOKEN_PROGRAM_ID);
            expect(accountInfo.mint).to.eql(mint);
            expect(accountInfo.owner).to.eql(owner.publicKey);
            expect(accountInfo.amount).to.eql(BigInt(0));
            expect(accountInfo.delegate).to.be.null;
            expect(accountInfo.delegatedAmount).to.eql(BigInt(0));
            expect(accountInfo.isInitialized).to.be.true;
            expect(accountInfo.isFrozen).to.be.false;
            expect(accountInfo.isNative).to.be.false;
            expect(accountInfo.rentExemptReserve).to.be.null;
            expect(accountInfo.closeAuthority).to.be.null;

            // you can create as many accounts as with same owner
            const account2 = await createAccount(
                connection,
                payer,
                mint,
                owner.publicKey,
                Keypair.generate(),
                undefined,
                TOKEN_PROGRAM_ID
            );
            expect(account2).to.not.eql(account);
        }),
        it('associated token account', async () => {
            const owner = Keypair.generate();
            const associatedAddress = await getAssociatedTokenAddress(
                mint,
                owner.publicKey,
                false,
                TOKEN_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID
            );

            // associated account shouldn't exist
            const info = await connection.getAccountInfo(associatedAddress);
            expect(info).to.be.null;

            const createdAddress = await createAccount(
                connection,
                payer,
                mint,
                owner.publicKey,
                undefined, // uses ATA by default
                undefined,
                TOKEN_PROGRAM_ID
            );
            expect(createdAddress).to.eql(associatedAddress);

            const accountInfo = await getAccount(connection, associatedAddress, undefined, TOKEN_PROGRAM_ID);
            expect(accountInfo).to.not.be.null;
            expect(accountInfo.mint).to.eql(mint);
            expect(accountInfo.owner).to.eql(owner.publicKey);
            expect(accountInfo.amount).to.eql(BigInt(0));

            // creating again should cause TX error for the associated token account
            expect(
                createAccount(
                    connection,
                    payer,
                    mint,
                    owner.publicKey,
                    undefined, // uses ATA by default
                    undefined,
                    TOKEN_PROGRAM_ID
                )
            ).to.be.rejected;
        });
});
