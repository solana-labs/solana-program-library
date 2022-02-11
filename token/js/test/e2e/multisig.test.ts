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
    mintTo,
    transfer,
    approve,
    getMultisig,
    createMultisig,
    setAuthority,
} from '../../src';

import { newAccountWithLamports, getConnection } from './common';

const TEST_TOKEN_DECIMALS = 2;
const M = 2;
const N = 5;
describe('multisig', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let account1: PublicKey;
    let account2: PublicKey;
    let amount: bigint;
    let multisig: PublicKey;
    let signers: Keypair[];
    let signerPublicKeys: PublicKey[];
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();
        signers = [];
        signerPublicKeys = [];
        for (let i = 0; i < N; ++i) {
            const signer = Keypair.generate();
            signers.push(signer);
            signerPublicKeys.push(signer.publicKey);
        }
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
        multisig = await createMultisig(connection, payer, signerPublicKeys, M, undefined, undefined, TOKEN_PROGRAM_ID);
        account1 = await createAccount(connection, payer, mint, multisig, undefined, undefined, TOKEN_PROGRAM_ID);
        account2 = await createAccount(
            connection,
            payer,
            mint,
            multisig,
            Keypair.generate(),
            undefined,
            TOKEN_PROGRAM_ID
        );
        amount = BigInt(1000);
        await mintTo(connection, payer, mint, account1, mintAuthority, amount, [], undefined, TOKEN_PROGRAM_ID);
    });
    it('create', async () => {
        const multisigInfo = await getMultisig(connection, multisig, undefined, TOKEN_PROGRAM_ID);
        expect(multisigInfo.m).to.eql(M);
        expect(multisigInfo.n).to.eql(N);
        expect(multisigInfo.signer1).to.eql(signerPublicKeys[0]);
        expect(multisigInfo.signer2).to.eql(signerPublicKeys[1]);
        expect(multisigInfo.signer3).to.eql(signerPublicKeys[2]);
        expect(multisigInfo.signer4).to.eql(signerPublicKeys[3]);
        expect(multisigInfo.signer5).to.eql(signerPublicKeys[4]);
    });
    it('transfer', async () => {
        await transfer(connection, payer, account1, account2, multisig, amount, signers, undefined, TOKEN_PROGRAM_ID);
        const accountInfo = await getAccount(connection, account2, undefined, TOKEN_PROGRAM_ID);
        expect(accountInfo.amount).to.eql(amount);
    });
    it('approve', async () => {
        const delegate = Keypair.generate().publicKey;
        await approve(connection, payer, account1, delegate, multisig, amount, signers, undefined, TOKEN_PROGRAM_ID);
        const approvedAccountInfo = await getAccount(connection, account1, undefined, TOKEN_PROGRAM_ID);
        expect(approvedAccountInfo.delegatedAmount).to.eql(amount);
        expect(approvedAccountInfo.delegate).to.eql(delegate);
    });
    it('setAuthority', async () => {
        const newOwner = Keypair.generate().publicKey;
        await setAuthority(
            connection,
            payer,
            account1,
            multisig,
            AuthorityType.AccountOwner,
            newOwner,
            signers,
            undefined,
            TOKEN_PROGRAM_ID
        );
        const accountInfo = await getAccount(connection, account1, undefined, TOKEN_PROGRAM_ID);
        expect(accountInfo.owner).to.eql(newOwner);
    });
});
