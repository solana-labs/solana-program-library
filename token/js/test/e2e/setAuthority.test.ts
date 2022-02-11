import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import { Connection, Keypair, PublicKey, Signer } from '@solana/web3.js';

import {
    TOKEN_PROGRAM_ID,
    AuthorityType,
    createMint,
    createAccount,
    getAccount,
    getMint,
    setAuthority,
} from '../../src';

import { newAccountWithLamports, getConnection } from './common';

const TEST_TOKEN_DECIMALS = 2;
describe('setAuthority', () => {
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
    });
    beforeEach(async () => {
        owner = Keypair.generate();
        account = await createAccount(connection, payer, mint, owner.publicKey, undefined, undefined, TOKEN_PROGRAM_ID);
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
            TOKEN_PROGRAM_ID
        );
        const accountInfo = await getAccount(connection, account, undefined, TOKEN_PROGRAM_ID);
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
            TOKEN_PROGRAM_ID
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
                TOKEN_PROGRAM_ID
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
            TOKEN_PROGRAM_ID
        );
        const mintInfo = await getMint(connection, mint, undefined, TOKEN_PROGRAM_ID);
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
            TOKEN_PROGRAM_ID
        );
        const accountInfo = await getAccount(connection, account, undefined, TOKEN_PROGRAM_ID);
        expect(accountInfo.closeAuthority).to.eql(closeAuthority.publicKey);
    });
});
