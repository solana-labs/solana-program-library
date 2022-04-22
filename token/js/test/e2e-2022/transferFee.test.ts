import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import {
    Connection,
    Keypair,
    PublicKey,
    Signer,
    SystemProgram,
    Transaction,
    sendAndConfirmTransaction,
} from '@solana/web3.js';

import {
    AuthorityType,
    setAuthority,
    ExtensionType,
    createInitializeAccountInstruction,
    createMint,
    getAccountLen,
    getTransferFee,
    getTransferFeeConfig,
    createAccount,
    getAccount,
} from '../../src';

import {
    createInitializeTransferFeeConfigInstruction,
    createTransferCheckedWithFeeInstruction,
    createWithdrawWithheldTokensFromMintInstruction,
    createWithdrawWithheldTokensFromAccountsInstruction,
    createHarvestWithheldTokensToMintInstruction,
} from '../../src/instructions/transferFee';

import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';
const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.ImmutableOwner];
describe('transferFee', () => {
    let connection: Connection;
    let payer: Signer;
    let owner: Keypair;
    let account: PublicKey;
    let mint: PublicKey;
    let transferFeeConfigAuthority: Keypair;
    let withdrawWithheldAuthority: Keypair;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        transferFeeConfigAuthority = Keypair.generate();
        withdrawWithheldAuthority = Keypair.generate();
    });
    beforeEach(async () => {
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
            TEST_PROGRAM_ID
        );
        owner = Keypair.generate();
        const accountLen = getAccountLen(EXTENSIONS);
        const lamports = await connection.getMinimumBalanceForRentExemption(accountLen);
        const accountKeypair = Keypair.generate();
        account = accountKeypair.publicKey;
        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: account,
                space: accountLen,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            /*createInitializeTransferFeeConfigInstruction(
                mint,
                transferFeeConfigAuthority.publicKey,
                withdrawWithheldAuthority.publicKey,
                100,
                BigInt('100000'),
                TEST_PROGRAM_ID
            ),*/
            createInitializeAccountInstruction(account, mint, owner.publicKey, TEST_PROGRAM_ID)
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, accountKeypair], undefined);
    });
    it('TransferFeeConfig', async () => {
        const owner = Keypair.generate();
        account = await createAccount(connection, payer, mint, owner.publicKey, undefined, undefined, TEST_PROGRAM_ID);
        //const _account = await getAccount(connection, account);
        //expect(getTransferFee(_account)).to.not.be.null;
    });
});
