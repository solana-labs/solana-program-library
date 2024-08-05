import { expect } from 'chai';
import type { Connection, Signer } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';

import {
    AuthorityType,
    ExtensionType,
    createInitializeGroupPointerInstruction,
    createInitializeMintInstruction,
    createSetAuthorityInstruction,
    createUpdateGroupPointerInstruction,
    getGroupPointerState,
    getMint,
    getMintLen,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.GroupPointer];

describe('Group pointer', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: Keypair;
    let mintAuthority: Keypair;
    let groupAddress: PublicKey;

    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
    });

    beforeEach(async () => {
        mint = Keypair.generate();
        groupAddress = PublicKey.unique();

        const mintLen = getMintLen(EXTENSIONS);
        const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mint.publicKey,
                space: mintLen,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeGroupPointerInstruction(
                mint.publicKey,
                mintAuthority.publicKey,
                groupAddress,
                TEST_PROGRAM_ID,
            ),
            createInitializeMintInstruction(
                mint.publicKey,
                TEST_TOKEN_DECIMALS,
                mintAuthority.publicKey,
                null,
                TEST_PROGRAM_ID,
            ),
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, mint], undefined);
    });

    it('can successfully initialize', async () => {
        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const groupPointer = getGroupPointerState(mintInfo);

        expect(groupPointer).to.deep.equal({
            authority: mintAuthority.publicKey,
            groupAddress,
        });
    });

    it('can update to new address', async () => {
        const newGroupAddress = PublicKey.unique();
        const transaction = new Transaction().add(
            createUpdateGroupPointerInstruction(
                mint.publicKey,
                mintAuthority.publicKey,
                newGroupAddress,
                undefined,
                TEST_PROGRAM_ID,
            ),
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, mintAuthority], undefined);

        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const groupPointer = getGroupPointerState(mintInfo);

        expect(groupPointer).to.deep.equal({
            authority: mintAuthority.publicKey,
            groupAddress: newGroupAddress,
        });
    });

    it('can update authority', async () => {
        const newAuthority = PublicKey.unique();
        const transaction = new Transaction().add(
            createSetAuthorityInstruction(
                mint.publicKey,
                mintAuthority.publicKey,
                AuthorityType.GroupPointer,
                newAuthority,
                [],
                TEST_PROGRAM_ID,
            ),
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, mintAuthority], undefined);

        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const groupPointer = getGroupPointerState(mintInfo);

        expect(groupPointer).to.deep.equal({
            authority: newAuthority,
            groupAddress,
        });
    });

    it('can update authority to null', async () => {
        const transaction = new Transaction().add(
            createSetAuthorityInstruction(
                mint.publicKey,
                mintAuthority.publicKey,
                AuthorityType.GroupPointer,
                null,
                [],
                TEST_PROGRAM_ID,
            ),
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, mintAuthority], undefined);

        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const groupPointer = getGroupPointerState(mintInfo);

        expect(groupPointer).to.deep.equal({
            authority: null,
            groupAddress,
        });
    });
});
