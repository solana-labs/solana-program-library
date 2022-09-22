import { Connection, Keypair, PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { SPL_NOOP_PROGRAM_ID } from "../utils";
import { getConcurrentMerkleTreeAccountSize } from '../accounts';
import {
    createReplaceLeafInstruction,
    createAppendInstruction,
    createTransferAuthorityInstruction,
    createVerifyLeafInstruction,
    PROGRAM_ID,
    createInitEmptyMerkleTreeInstruction
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

export function createInitEmptyMerkleTreeIx(
    authority: Keypair,
    merkleTree: PublicKey,
    maxDepth: number,
    maxBufferSize: number
): TransactionInstruction {
    return createInitEmptyMerkleTreeInstruction(
        {
            merkleTree,
            authority: authority.publicKey,
            logWrapper: SPL_NOOP_PROGRAM_ID,
        },
        {
            maxBufferSize,
            maxDepth
        }
    );
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
            logWrapper: SPL_NOOP_PROGRAM_ID,
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
            logWrapper: SPL_NOOP_PROGRAM_ID,
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

export async function createAllocTreeIx(
    connection: Connection,
    maxBufferSize: number,
    maxDepth: number,
    canopyDepth: number,
    payer: PublicKey,
    merkleTree: PublicKey,
): Promise<TransactionInstruction> {
    const requiredSpace = getConcurrentMerkleTreeAccountSize(
        maxDepth,
        maxBufferSize,
        canopyDepth ?? 0
    );
    return SystemProgram.createAccount({
        fromPubkey: payer,
        newAccountPubkey: merkleTree,
        lamports:
            await connection.getMinimumBalanceForRentExemption(
                requiredSpace
            ),
        space: requiredSpace,
        programId: PROGRAM_ID
    });
}