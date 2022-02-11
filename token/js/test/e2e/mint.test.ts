import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import { Connection, Keypair, PublicKey, Signer } from '@solana/web3.js';

import { TOKEN_PROGRAM_ID, createMint, getMint, createAccount, getAccount, mintTo, mintToChecked } from '../../src';

import { newAccountWithLamports, getConnection } from './common';

const TEST_TOKEN_DECIMALS = 2;
describe('mint', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let owner: Keypair;
    let account: PublicKey;
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
            TOKEN_PROGRAM_ID
        );
        owner = Keypair.generate();
        account = await createAccount(connection, payer, mint, owner.publicKey, undefined, undefined, TOKEN_PROGRAM_ID);
    });
    it('mintTo', async () => {
        const amount = BigInt(1000);
        await mintTo(connection, payer, mint, account, mintAuthority, amount, [], undefined, TOKEN_PROGRAM_ID);

        const mintInfo = await getMint(connection, mint, undefined, TOKEN_PROGRAM_ID);
        expect(mintInfo.supply).to.eql(amount);

        const accountInfo = await getAccount(connection, account, undefined, TOKEN_PROGRAM_ID);
        expect(accountInfo.amount).to.eql(amount);
    });
    it('mintToChecked', async () => {
        const amount = BigInt(1000);
        await mintToChecked(
            connection,
            payer,
            mint,
            account,
            mintAuthority,
            amount,
            TEST_TOKEN_DECIMALS,
            [],
            undefined,
            TOKEN_PROGRAM_ID
        );

        expect(
            mintToChecked(connection, payer, mint, account, mintAuthority, amount, 1, [], undefined, TOKEN_PROGRAM_ID)
        ).to.be.rejected;
    });
});
