import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { Keypair } from '@solana/web3.js';

import { AuthorityType, createMint, createAccount, getAccount, getMint, setAuthority } from '../../src';

import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
describe('setAuthority', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let freezeAuthority: Keypair;
    let owner: Keypair;
    let account: PublicKey;
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
        account = await createAccount(
            connection,
            payer,
            mint,
            owner.publicKey,
            Keypair.generate(),
            undefined,
            TEST_PROGRAM_ID
        );
    });
    it('AccountOwner', async () => {
        const newOwner = Keypair.generate();
        await setAuthority(
            connection,
            payer,
            account,
            owner,
            AuthorityType.AccountOwner,
            newOwner.publicKey,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
        expect(accountInfo.owner).to.eql(newOwner.publicKey);
        await setAuthority(
            connection,
            payer,
            account,
            newOwner,
            AuthorityType.AccountOwner,
            owner.publicKey,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        expect(
            setAuthority(
                connection,
                payer,
                account,
                newOwner,
                AuthorityType.AccountOwner,
                owner.publicKey,
                [],
                undefined,
                TEST_PROGRAM_ID
            )
        ).to.be.rejected;
    });
    it('MintAuthority', async () => {
        await setAuthority(
            connection,
            payer,
            mint,
            mintAuthority,
            AuthorityType.MintTokens,
            null,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        expect(mintInfo.mintAuthority).to.be.null;
    });
    it('CloseAuthority', async () => {
        const closeAuthority = Keypair.generate();
        await setAuthority(
            connection,
            payer,
            account,
            owner,
            AuthorityType.CloseAccount,
            closeAuthority.publicKey,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
        expect(accountInfo.closeAuthority).to.eql(closeAuthority.publicKey);
    });
    it('FreezeAuthority', async () => {
        await setAuthority(
            connection,
            payer,
            mint,
            freezeAuthority,
            AuthorityType.FreezeAccount,
            null,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        expect(mintInfo.freezeAuthority).to.be.null;
    });
});
