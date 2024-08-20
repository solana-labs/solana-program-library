import { expect } from 'chai';
import type { Connection, Signer } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';

import {
    ExtensionType,
    createInitializeMintInstruction,
    createInitializeGroupInstruction,
    getTokenGroupState,
    getMint,
    getMintLen,
    createInitializeGroupMemberPointerInstruction,
    createInitializeGroupPointerInstruction,
    getTokenGroupMemberState,
    tokenGroupInitializeGroupWithRentTransfer,
    tokenGroupMemberInitializeWithRentTransfer,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;

describe('tokenGroupMember', async () => {
    let connection: Connection;
    let payer: Signer;

    let groupMint: Keypair;
    let groupUpdateAuthority: Keypair;

    let memberMint: Keypair;
    let memberMintAuthority: Keypair;
    let memberUpdateAuthority: Keypair;

    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);

        groupMint = Keypair.generate();
        const groupMintAuthority = Keypair.generate();
        groupUpdateAuthority = Keypair.generate();

        memberMint = Keypair.generate();
        memberMintAuthority = Keypair.generate();
        memberUpdateAuthority = Keypair.generate();

        const groupMintLen = getMintLen([ExtensionType.GroupPointer]);
        const groupMintLamports = await connection.getMinimumBalanceForRentExemption(groupMintLen);

        const memberMintLen = getMintLen([ExtensionType.GroupMemberPointer]);
        const memberMintLamports = await connection.getMinimumBalanceForRentExemption(memberMintLen);

        // Create the group mint and initialize the group.
        await sendAndConfirmTransaction(
            connection,
            new Transaction().add(
                SystemProgram.createAccount({
                    fromPubkey: payer.publicKey,
                    newAccountPubkey: groupMint.publicKey,
                    space: groupMintLen,
                    lamports: groupMintLamports,
                    programId: TEST_PROGRAM_ID,
                }),
                createInitializeGroupPointerInstruction(
                    groupMint.publicKey,
                    groupUpdateAuthority.publicKey,
                    groupMint.publicKey,
                    TEST_PROGRAM_ID,
                ),
                createInitializeMintInstruction(
                    groupMint.publicKey,
                    TEST_TOKEN_DECIMALS,
                    groupMintAuthority.publicKey,
                    null,
                    TEST_PROGRAM_ID,
                ),
            ),
            [payer, groupMint],
            undefined,
        );
        await tokenGroupInitializeGroupWithRentTransfer(
            connection,
            payer,
            groupMint.publicKey,
            groupMintAuthority.publicKey,
            groupUpdateAuthority.publicKey,
            BigInt(3),
            [payer, groupMintAuthority],
            undefined,
            TEST_PROGRAM_ID,
        );

        // Create the member mint.
        await sendAndConfirmTransaction(
            connection,
            new Transaction().add(
                SystemProgram.createAccount({
                    fromPubkey: payer.publicKey,
                    newAccountPubkey: memberMint.publicKey,
                    space: memberMintLen,
                    lamports: memberMintLamports,
                    programId: TEST_PROGRAM_ID,
                }),
                createInitializeGroupMemberPointerInstruction(
                    memberMint.publicKey,
                    memberUpdateAuthority.publicKey,
                    memberMint.publicKey,
                    TEST_PROGRAM_ID,
                ),
                createInitializeMintInstruction(
                    memberMint.publicKey,
                    TEST_TOKEN_DECIMALS,
                    memberMintAuthority.publicKey,
                    null,
                    TEST_PROGRAM_ID,
                ),
            ),
            [payer, memberMint],
            undefined,
        );
    });

    it('can initialize a group member', async () => {
        const tokenGroupMember = {
            mint: memberMint.publicKey,
            group: groupMint.publicKey,
            memberNumber: BigInt(1),
        };

        await tokenGroupMemberInitializeWithRentTransfer(
            connection,
            payer,
            memberMint.publicKey,
            memberMintAuthority.publicKey,
            groupMint.publicKey,
            groupUpdateAuthority.publicKey,
            [memberMintAuthority, groupUpdateAuthority],
            undefined,
            TEST_PROGRAM_ID,
        );

        const mintInfo = await getMint(connection, memberMint.publicKey, undefined, TEST_PROGRAM_ID);
        const member = getTokenGroupMemberState(mintInfo);
        expect(member).to.deep.equal(tokenGroupMember);
    });
});
