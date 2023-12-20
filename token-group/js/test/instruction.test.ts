import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { expect } from 'chai';

import {
    createInitializeGroupInstruction,
    createInitializeMemberInstruction,
    createUpdateGroupMaxSizeInstruction,
    createUpdateGroupAuthorityInstruction,
} from '../src';

describe('Token Group Instructions', () => {
    const programId = new PublicKey('22222222222222222222222222222222222222222222');
    const group = new PublicKey('33333333333333333333333333333333333333333333');
    const updateAuthority = new PublicKey('44444444444444444444444444444444444444444444');
    const mint = new PublicKey('55555555555555555555555555555555555555555555');
    const mintAuthority = new PublicKey('66666666666666666666666666666666666666666666');
    const maxSize = 100;

    it('Can create InitializeGroup Instruction', () => {
        const instruction = createInitializeGroupInstruction({
            programId,
            group,
            mint,
            mintAuthority,
            updateAuthority,
            maxSize,
        });

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: group },
                    { isSigner: false, isWritable: false, pubkey: mint },
                    { isSigner: true, isWritable: false, pubkey: mintAuthority },
                ],
                data: Buffer.from([
                    // Output of rust implementation
                    121, 113, 108, 39, 54, 51, 0, 4, 45, 91, 65, 60, 101, 64, 222, 21, 12, 147, 115, 20, 77, 81, 51,
                    202, 76, 184, 48, 186, 15, 117, 103, 22, 172, 234, 14, 80, 215, 148, 53, 229, 100, 0, 0, 0,
                ]),
            })
        );
    });

    it('Can create UpdateGroupMaxSize Instruction', () => {
        const instruction = createUpdateGroupMaxSizeInstruction({
            programId,
            group,
            updateAuthority,
            maxSize,
        });

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: group },
                    { isSigner: true, isWritable: false, pubkey: updateAuthority },
                ],
                data: Buffer.from([
                    // Output of rust implementation
                    108, 37, 171, 143, 248, 30, 18, 110, 100, 0, 0, 0,
                ]),
            })
        );
    });

    it('Can create UpdateGroupAuthority Instruction', () => {
        const instruction = createUpdateGroupAuthorityInstruction({
            programId,
            group,
            currentAuthority: updateAuthority,
            newAuthority: PublicKey.default,
        });

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: group },
                    { isSigner: true, isWritable: false, pubkey: updateAuthority },
                ],
                data: Buffer.from([
                    // Output of rust implementation
                    161, 105, 88, 1, 237, 221, 216, 203, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ]),
            })
        );
    });

    it('Can create InitializeMember Instruction', () => {
        const member = new PublicKey('22222222222222222222222222222222222222222222');
        const memberMint = new PublicKey('33333333333333333333333333333333333333333333');
        const memberMintAuthority = new PublicKey('44444444444444444444444444444444444444444444');
        const group = new PublicKey('55555555555555555555555555555555555555555555');
        const groupUpdateAuthority = new PublicKey('66666666666666666666666666666666666666666666');

        const instruction = createInitializeMemberInstruction({
            programId,
            member,
            memberMint,
            memberMintAuthority,
            group,
            groupUpdateAuthority,
        });

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: member },
                    { isSigner: false, isWritable: false, pubkey: memberMint },
                    { isSigner: true, isWritable: false, pubkey: memberMintAuthority },
                    { isSigner: false, isWritable: true, pubkey: group },
                    { isSigner: true, isWritable: false, pubkey: groupUpdateAuthority },
                ],
                data: Buffer.from([152, 32, 222, 176, 223, 237, 116, 134]),
            })
        );
    });
});
