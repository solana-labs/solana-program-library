import { Program } from "@project-serum/anchor";
import { Keypair, PublicKey, TransactionInstruction } from '@solana/web3.js';
import { LOG_WRAPPER_PROGRAM_ID } from "../utils";
import {
    createReplaceLeafInstruction,
    createAppendInstruction,
    createTransferAuthorityInstruction,
    createVerifyLeafInstruction
} from "../generated";

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
    authority: Keypair,
    merkleTree: PublicKey,
    treeRoot: Buffer,
    previousLeaf: Buffer,
    newLeaf: Buffer,
    index: number,
    proof: Buffer[]
): TransactionInstruction {
    return addProof(createReplaceLeafInstruction(
        {
            merkleTree,
            authority: authority.publicKey,
            logWrapper: LOG_WRAPPER_PROGRAM_ID,
        },
        {
            root: Array.from(treeRoot),
            previousLeaf: Array.from(previousLeaf),
            newLeaf: Array.from(newLeaf),
            index,
        }
    ), proof);
}

export function createAppendIx(
    newLeaf: Buffer | ArrayLike<number>,
    authority: Keypair,
    merkleTree: PublicKey,
): TransactionInstruction {
    return createAppendInstruction(
        {
            merkleTree,
            authority: authority.publicKey,
            logWrapper: LOG_WRAPPER_PROGRAM_ID,
        },
        {
            leaf: Array.from(newLeaf),
        }
    )
}

export function createTransferAuthorityIx(
    authority: Keypair,
    merkleTree: PublicKey,
    newAuthority: PublicKey,
): TransactionInstruction {
    return createTransferAuthorityInstruction(
        {
            merkleTree,
            authority: authority.publicKey,
        },
        {
            newAuthority,
        }
    );
}

export function createVerifyLeafIx(
    merkleTree: PublicKey,
    root: Buffer,
    leaf: Buffer,
    index: number,
    proof: Buffer[],
): TransactionInstruction {
    return addProof(createVerifyLeafInstruction(
        {
            merkleTree
        },
        {
            root: Array.from(root),
            leaf: Array.from(leaf),
            index,
        }
    ), proof);
}
