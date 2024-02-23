import { expect } from 'chai';
import type { Connection, Signer } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';

import {
    ExtensionType,
    createInitializeGroupMemberPointerInstruction,
    createInitializeMintInstruction,
    createUpdateGroupMemberPointerInstruction,
    getGroupMemberPointerState,
    getMint,
    getMintLen,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.GroupMemberPointer];

describe('GroupMember pointer', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: Keypair;
    let mintAuthority: Keypair;
    let memberAddress: PublicKey;

    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
    });

    beforeEach(async () => {
        mint = Keypair.generate();
        memberAddress = PublicKey.unique();

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
            createInitializeGroupMemberPointerInstruction(
                mint.publicKey,
                mintAuthority.publicKey,
                memberAddress,
                TEST_PROGRAM_ID
            ),
            createInitializeMintInstruction(
                mint.publicKey,
                TEST_TOKEN_DECIMALS,
                mintAuthority.publicKey,
                null,
                TEST_PROGRAM_ID
            )
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, mint], undefined);
    });

    it('can successfully initialize', async () => {
        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const groupMemberPointer = getGroupMemberPointerState(mintInfo);

        expect(groupMemberPointer).to.deep.equal({
            authority: mintAuthority.publicKey,
            memberAddress,
        });
    });

    it('can update to new address', async () => {
        const newGroupMemberAddress = PublicKey.unique();
        const transaction = new Transaction().add(
            createUpdateGroupMemberPointerInstruction(
                mint.publicKey,
                mintAuthority.publicKey,
                newGroupMemberAddress,
                undefined,
                TEST_PROGRAM_ID
            )
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, mintAuthority], undefined);

        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const groupMemberPointer = getGroupMemberPointerState(mintInfo);

        expect(groupMemberPointer).to.deep.equal({
            authority: mintAuthority.publicKey,
            memberAddress: newGroupMemberAddress,
        });
    });
});
