import { expect } from 'chai';
import type { Connection, Signer } from '@solana/web3.js';
import type { PublicKey } from '@solana/web3.js';
import { Keypair, SystemProgram, Transaction, sendAndConfirmTransaction } from '@solana/web3.js';
import { ElGamalKeypair, PodElGamalPubkey } from 'solana-zk-token-sdk-experimental';
import { ExtensionType, createInitializeMintInstruction, getMint, getMintLen } from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

import {
    createConfidentialTransferInitializeMintInstruction,
    getConfidentialTransferMint,
    updateMint,
} from '../../src/extensions/confidentialTransfer/index';

const TEST_TOKEN_DECIMALS = 2;
const MINT_EXTENSIONS = [ExtensionType.ConfidentialTransferMint];

describe('confidentialTransfer', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
    });

    async function setupConfidentialTransferMint(
        confidentialTransferMintAuthority: PublicKey | null,
        autoApproveNewAccounts: boolean,
        auditorPubkey: PodElGamalPubkey | null
    ) {
        const mintKeypair = Keypair.generate();
        mint = mintKeypair.publicKey;
        mintAuthority = Keypair.generate();
        const mintLen = getMintLen(MINT_EXTENSIONS);

        const mintLamports = await connection.getMinimumBalanceForRentExemption(mintLen);
        const mintTransaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mint,
                space: mintLen,
                lamports: mintLamports,
                programId: TEST_PROGRAM_ID,
            }),
            createConfidentialTransferInitializeMintInstruction(
                mint,
                confidentialTransferMintAuthority,
                autoApproveNewAccounts,
                auditorPubkey
            ),
            createInitializeMintInstruction(mint, TEST_TOKEN_DECIMALS, mintAuthority.publicKey, null, TEST_PROGRAM_ID)
        );

        await sendAndConfirmTransaction(connection, mintTransaction, [payer, mintKeypair], undefined);
    }

    describe('with authorities and auto approve', () => {
        let confidentialTransferMintAuthority: Keypair;
        let autoApproveNewAccounts: boolean;
        let auditorKeypair: ElGamalKeypair;
        let auditorPubkey: PodElGamalPubkey;
        beforeEach(async () => {
            confidentialTransferMintAuthority = Keypair.generate();
            autoApproveNewAccounts = true;
            auditorKeypair = ElGamalKeypair.new_rand();
            auditorPubkey = PodElGamalPubkey.encoded(auditorKeypair.pubkey_owned());

            await setupConfidentialTransferMint(
                confidentialTransferMintAuthority.publicKey,
                autoApproveNewAccounts,
                auditorPubkey
            );
        });

        it('initializes', async () => {
            const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
            const confidentialTransferMint = getConfidentialTransferMint(mintInfo);
            expect(confidentialTransferMint).to.not.be.null;
            if (confidentialTransferMint !== null) {
                expect(confidentialTransferMint.confidentialTransferMintAuthority).to.eql(
                    confidentialTransferMintAuthority.publicKey
                );
                expect(confidentialTransferMint.autoApproveNewAccounts).to.eql(autoApproveNewAccounts);
                expect(confidentialTransferMint.auditorElGamalPubkey.equals(auditorPubkey)); // TODO: equals?
            }
        });

        it('mint updates', async () => {
            const newAutoApproveNewAccounts = false;
            const newAuditorElGamalKeypair = ElGamalKeypair.new_rand();
            const newAuditorElGamalPubkey = PodElGamalPubkey.encoded(newAuditorElGamalKeypair.pubkey_owned());

            await updateMint(
                connection,
                payer,
                mint,
                newAutoApproveNewAccounts,
                newAuditorElGamalPubkey,
                confidentialTransferMintAuthority
            );

            const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);
            const confidentialTransferMint = getConfidentialTransferMint(mintInfo);
            expect(confidentialTransferMint).to.not.be.null;
            if (confidentialTransferMint !== null) {
                expect(confidentialTransferMint.autoApproveNewAccounts).to.eql(newAutoApproveNewAccounts);
                expect(confidentialTransferMint.auditorElGamalPubkey.equals(newAuditorElGamalPubkey));
            }
        });
    });
});
