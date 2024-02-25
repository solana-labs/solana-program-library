import { expect } from 'chai';
import type { Connection, Signer } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';
import { TokenGroup, packTokenGroup, packTokenGroupMember } from '@solana/spl-token-group';

import {
    ExtensionType,
    createInitializeMintInstruction,
    createInitializeGroupInstruction,
    tokenGroupMemberInitialize,
    getTokenGroupState,
    getMint,
    getMintLen,
    createInitializeGroupMemberPointerInstruction,
    createInitializeGroupPointerInstruction,
    getTokenGroupMemberState,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.GroupMemberPointer, ExtensionType.GroupPointer];

describe.only('tokenGroupMember', async () => {
    let connection: Connection;
    let payer: Signer;
    let mint: Keypair;
    let mintAuthority: Keypair;
    let updateAuthority: Keypair;
    let groupAddress: PublicKey;
    let memberAddress: PublicKey;
    let memberMintAuthority: Keypair;

    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
        updateAuthority = Keypair.generate();
        memberMintAuthority = Keypair.generate();
    });

    beforeEach(async () => {
        mint = Keypair.generate();
        groupAddress = PublicKey.unique();
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
            createInitializeGroupPointerInstruction(
                mint.publicKey,
                mintAuthority.publicKey,
                groupAddress,
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

    it('can initialize', async () => {
        const tokenGroupMember = {
            mint: mint.publicKey,
            group: groupAddress,
            memberNumber: 1,
        };

        // Transfer the required amount for rent exemption
        const lamports = await connection.getMinimumBalanceForRentExemption(
            packTokenGroupMember(tokenGroupMember).length
        );
        const transaction = new Transaction().add(
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: mint.publicKey,
                lamports,
            })
        );
        await sendAndConfirmTransaction(connection, transaction, [payer], undefined);

        await tokenGroupMemberInitialize(
            connection,
            payer,
            mint.publicKey,
            memberAddress,
            memberMintAuthority.publicKey,
            groupAddress,
            updateAuthority.publicKey,
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );

        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const member = getTokenGroupMemberState(mintInfo);
        expect(member).to.deep.equal({
            mint: mint.publicKey,
            group: groupAddress,
            memberNumber: 1,
        });
    });
});
