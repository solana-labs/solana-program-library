import { Program } from '@project-serum/anchor';
import { Gummyroll } from "../types";
import { Keypair, PublicKey, TransactionInstruction } from '@solana/web3.js';
import { CANDY_WRAPPER_PROGRAM_ID } from '../../utils'

/**
 * Modifies given instruction
 */
export function addProof(
    instruction: TransactionInstruction,
    nodeProof: Buffer[],
): TransactionInstruction {
    instruction.keys = instruction.keys.concat(
        nodeProof.map((node) => {
            return {
                pubkey: new PublicKey(node),
                isSigner: false,
                isWritable: false,
            };
        })
    )
    return instruction;
}

export function createReplaceIx(
    gummyroll: Program<Gummyroll>,
    authority: Keypair,
    merkleRoll: PublicKey,
    treeRoot: Buffer,
    previousLeaf: Buffer,
    newLeaf: Buffer,
    index: number,
    proof: Buffer[]
): TransactionInstruction {
    return addProof(gummyroll.instruction.replaceLeaf(
        Array.from(treeRoot),
        Array.from(previousLeaf),
        Array.from(newLeaf),
        index,
        {
            accounts: {
                merkleRoll,
                authority: authority.publicKey,
                candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
            },
            signers: [authority],
        }
    ), proof);
}

export function createAppendIx(
    gummyroll: Program<Gummyroll>,
    newLeaf: Buffer | ArrayLike<number>,
    authority: Keypair,
    appendAuthority: Keypair,
    merkleRoll: PublicKey,
): TransactionInstruction {
    return gummyroll.instruction.append(
        Array.from(newLeaf),
        {
            accounts: {
                merkleRoll,
                authority: authority.publicKey,
                appendAuthority: appendAuthority.publicKey,
                candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
            },
            signers: [authority, appendAuthority],
        }
    );
}

export function createTransferAuthorityIx(
    gummyroll: Program<Gummyroll>,
    authority: Keypair,
    merkleRoll: PublicKey,
    newAuthority: PublicKey | null,
    newAppendAuthority: PublicKey | null,
): TransactionInstruction {
    return gummyroll.instruction.transferAuthority(
        newAuthority,
        newAppendAuthority,
        {
            accounts: {
                merkleRoll,
                authority: authority.publicKey,
            },
            signers: [authority],
        }
    );
}

export function createVerifyLeafIx(
    gummyroll: Program<Gummyroll>,
    merkleRoll: PublicKey,
    root: Buffer,
    leaf: Buffer,
    index: number,
    proof: Buffer[],
): TransactionInstruction {
    return addProof(gummyroll.instruction.verifyLeaf(
        Array.from(root),
        Array.from(leaf),
        index,
        {
            accounts: {
                merkleRoll
            },
            signers: [],
        }
    ), proof);
}
