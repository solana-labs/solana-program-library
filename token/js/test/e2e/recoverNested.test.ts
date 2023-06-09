import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { Keypair, Transaction, sendAndConfirmTransaction } from '@solana/web3.js';

import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createMint,
    getAccount,
    createAssociatedTokenAccount,
    createAssociatedTokenAccountInstruction,
    getAssociatedTokenAddressSync,
    mintTo,
    recoverNested,
} from '../../src';

import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

describe('recoverNested', () => {
    let connection: Connection;
    let payer: Signer;
    let owner: Signer;
    let mint: PublicKey;
    let associatedToken: PublicKey;
    let nestedMint: PublicKey;
    const nestedMintAmount = 1;
    let nestedAssociatedToken: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 10000000000);
        owner = Keypair.generate();

        // mint
        const mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();
        mint = await createMint(
            connection,
            payer,
            mintAuthority.publicKey,
            mintAuthority.publicKey,
            0,
            mintKeypair,
            undefined,
            TEST_PROGRAM_ID
        );

        associatedToken = await createAssociatedTokenAccount(
            connection,
            payer,
            mint,
            owner.publicKey,
            undefined,
            TEST_PROGRAM_ID
        );

        // nested mint
        const nestedMintAuthority = Keypair.generate();
        const nestedMintKeypair = Keypair.generate();
        nestedMint = await createMint(
            connection,
            payer,
            nestedMintAuthority.publicKey,
            nestedMintAuthority.publicKey,
            0,
            nestedMintKeypair,
            undefined,
            TEST_PROGRAM_ID
        );

        nestedAssociatedToken = getAssociatedTokenAddressSync(
            nestedMint,
            associatedToken,
            true,
            TEST_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );
        const transaction = new Transaction().add(
            createAssociatedTokenAccountInstruction(
                payer.publicKey,
                nestedAssociatedToken,
                associatedToken,
                nestedMint,
                TEST_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID
            )
        );
        await sendAndConfirmTransaction(connection, transaction, [payer], undefined);

        // use mintTo to make nestedAssociatedToken have some tokens
        await mintTo(
            connection,
            payer,
            nestedMint,
            nestedAssociatedToken,
            nestedMintAuthority,
            nestedMintAmount,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );
    }),
        it('success', async () => {
            // create destinaion associated token
            const destinationAssociatedToken = await createAssociatedTokenAccount(
                connection,
                payer,
                nestedMint,
                owner.publicKey,
                undefined,
                TEST_PROGRAM_ID
            );

            await recoverNested(connection, payer, owner, mint, nestedMint, undefined, TEST_PROGRAM_ID);

            expect(await connection.getAccountInfo(nestedAssociatedToken)).to.be.null;

            const accountInfo = await getAccount(connection, destinationAssociatedToken, undefined, TEST_PROGRAM_ID);
            expect(accountInfo).to.not.be.null;
            expect(accountInfo.mint).to.eql(nestedMint);
            expect(accountInfo.owner).to.eql(owner.publicKey);
            expect(accountInfo.amount).to.eql(BigInt(nestedMintAmount));
        });
});
