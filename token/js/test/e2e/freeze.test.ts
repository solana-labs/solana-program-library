import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { Keypair } from '@solana/web3.js';

import { burn, createMint, createAccount, getAccount, freezeAccount, thawAccount, mintTo } from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
describe('freezeThaw', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let freezeAuthority: Keypair;
    let owner: Keypair;
    let account: PublicKey;
    let amount: bigint;
    const burnAmount = BigInt(1);
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
        account = await createAccount(connection, payer, mint, owner.publicKey, undefined, undefined, TEST_PROGRAM_ID);
        amount = BigInt(1000);
        await mintTo(connection, payer, mint, account, mintAuthority, amount, [], undefined, TEST_PROGRAM_ID);
    });
    it('freezes', async () => {
        await freezeAccount(connection, payer, account, mint, freezeAuthority, [], undefined, TEST_PROGRAM_ID);

        expect(burn(connection, payer, account, mint, owner, burnAmount, [], undefined, TEST_PROGRAM_ID)).to.be
            .rejected;
    });
    it('thaws', async () => {
        await freezeAccount(connection, payer, account, mint, freezeAuthority, [], undefined, TEST_PROGRAM_ID);

        await thawAccount(connection, payer, account, mint, freezeAuthority, [], undefined, TEST_PROGRAM_ID);

        await burn(connection, payer, account, mint, owner, burnAmount, [], undefined, TEST_PROGRAM_ID);
        const accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);

        expect(accountInfo.amount).to.eql(amount - burnAmount);
    });
});
