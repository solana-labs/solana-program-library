import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import {
    Connection,
    Keypair,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    SystemProgram,
    Transaction,
} from '@solana/web3.js';
import {
    createInitializeInterestBearingMintInstruction,
    createInitializeMintInstruction,
    ExtensionType,
    getInterestBearingMintConfigState,
    getMint,
    getMintLen,
} from '../../src';
import { getConnection, newAccountWithLamports, TEST_PROGRAM_ID } from '../common';

const TEST_TOKEN_DECIMALS = 2;

describe('interestBearingMint', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let rateAuthority: Keypair;
    let mintAuthority: Keypair;
    let freezeAuthority: Keypair;

    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        rateAuthority = Keypair.generate();
        mintAuthority = Keypair.generate();
        freezeAuthority = Keypair.generate();
    });

    beforeEach(async () => {
        const mintKeypair = Keypair.generate();
        mint = mintKeypair.publicKey;
        const mintLen = getMintLen([ExtensionType.InterestBearingMint]);
        const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);
        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mint,
                space: mintLen,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeInterestBearingMintInstruction(mint, rateAuthority.publicKey, 10, TEST_PROGRAM_ID),
            createInitializeMintInstruction(
                mint,
                TEST_TOKEN_DECIMALS,
                mintAuthority.publicKey,
                freezeAuthority.publicKey,
                TEST_PROGRAM_ID
            )
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair], undefined);
    });

    it('initialized with correct params', async () => {
        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
        const interestBearingMintConfigState = getInterestBearingMintConfigState(mintInfo);
        expect(interestBearingMintConfigState).to.not.be.null;
        if (interestBearingMintConfigState !== null) {
            expect(interestBearingMintConfigState.currentRate).to.eql(10);
        }
    });
});
