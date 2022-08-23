import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { Keypair } from '@solana/web3.js';

import {
    createMint,
    createAccount,
    getAccount,
    mintTo,
    transfer,
    transferChecked,
    approve,
    approveChecked,
    revoke,
} from '../../src';

import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
describe('transfer', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let owner1: Keypair;
    let account1: PublicKey;
    let owner2: Keypair;
    let account2: PublicKey;
    let amount: bigint;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
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
    });
    beforeEach(async () => {
        owner1 = Keypair.generate();
        account1 = await createAccount(
            connection,
            payer,
            mint,
            owner1.publicKey,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );
        owner2 = Keypair.generate();
        account2 = await createAccount(
            connection,
            payer,
            mint,
            owner2.publicKey,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );
        amount = BigInt(1000);
        await mintTo(connection, payer, mint, account1, mintAuthority, amount, [], undefined, TEST_PROGRAM_ID);
    });
    it('transfer', async () => {
        await transfer(connection, payer, account1, account2, owner1, amount, [], undefined, TEST_PROGRAM_ID);

        const destAccountInfo = await getAccount(connection, account2, undefined, TEST_PROGRAM_ID);
        expect(destAccountInfo.amount).to.eql(amount);

        const sourceAccountInfo = await getAccount(connection, account1, undefined, TEST_PROGRAM_ID);
        expect(sourceAccountInfo.amount).to.eql(BigInt(0));
    });
    it('transferChecked', async () => {
        const transferAmount = amount / BigInt(2);
        await transferChecked(
            connection,
            payer,
            account1,
            mint,
            account2,
            owner1,
            transferAmount,
            TEST_TOKEN_DECIMALS,
            [],
            undefined,
            TEST_PROGRAM_ID
        );

        const destAccountInfo = await getAccount(connection, account2, undefined, TEST_PROGRAM_ID);
        expect(destAccountInfo.amount).to.eql(transferAmount);

        const sourceAccountInfo = await getAccount(connection, account1, undefined, TEST_PROGRAM_ID);
        expect(sourceAccountInfo.amount).to.eql(transferAmount);
        expect(
            transferChecked(
                connection,
                payer,
                account1,
                mint,
                account2,
                owner1,
                transferAmount,
                TEST_TOKEN_DECIMALS - 1,
                [],
                undefined,
                TEST_PROGRAM_ID
            )
        ).to.be.rejected;
    });
    it('approveRevoke', async () => {
        const delegate = Keypair.generate();
        const delegatedAmount = amount / BigInt(2);
        await approve(
            connection,
            payer,
            account1,
            delegate.publicKey,
            owner1,
            delegatedAmount,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const approvedAccountInfo = await getAccount(connection, account1, undefined, TEST_PROGRAM_ID);
        expect(approvedAccountInfo.delegatedAmount).to.eql(delegatedAmount);
        expect(approvedAccountInfo.delegate).to.eql(delegate.publicKey);
        await revoke(connection, payer, account1, owner1, [], undefined, TEST_PROGRAM_ID);
        const revokedAccountInfo = await getAccount(connection, account1, undefined, TEST_PROGRAM_ID);
        expect(revokedAccountInfo.delegatedAmount).to.eql(BigInt(0));
        expect(revokedAccountInfo.delegate).to.be.null;
    });
    it('delegateTransfer', async () => {
        const delegate = Keypair.generate();
        const delegatedAmount = amount / BigInt(2);
        await approveChecked(
            connection,
            payer,
            mint,
            account1,
            delegate.publicKey,
            owner1,
            delegatedAmount,
            TEST_TOKEN_DECIMALS,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const transferAmount = delegatedAmount - BigInt(1);
        await transfer(connection, payer, account1, account2, delegate, transferAmount, [], undefined, TEST_PROGRAM_ID);
        const accountInfo = await getAccount(connection, account1, undefined, TEST_PROGRAM_ID);
        expect(accountInfo.delegatedAmount).to.eql(delegatedAmount - transferAmount);
        expect(accountInfo.delegate).to.eql(delegate.publicKey);
        expect(transfer(connection, payer, account1, account2, delegate, BigInt(2), [], undefined, TEST_PROGRAM_ID)).to
            .be.rejected;
    });
});
