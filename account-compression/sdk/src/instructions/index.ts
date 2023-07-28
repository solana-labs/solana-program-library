import { Connection, PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';

import { getConcurrentMerkleTreeAccountSize } from '../accounts';
import { SPL_NOOP_PROGRAM_ID, ValidDepthSizePair } from '../constants';
import {
    createAppendInstruction,
    createCloseEmptyTreeInstruction,
    createInitEmptyMerkleTreeInstruction,
    createReplaceLeafInstruction,
    createTransferAuthorityInstruction,
    createVerifyLeafInstruction,
    PROGRAM_ID,
} from '../generated';
import { MerkleTreeProof } from '../merkle-tree';

/**
 * Helper function that adds proof nodes to a TransactionInstruction
 * by adding extra keys to the transaction
 */
export function addProof(instruction: TransactionInstruction, nodeProof: Buffer[]): TransactionInstruction {
    instruction.keys = instruction.keys.concat(
        nodeProof.map(node => {
            return {
                isSigner: false,
                isWritable: false,
                pubkey: new PublicKey(node),
            };
        })
    );
    return instruction;
}

/**
 * Helper function for {@link createInitEmptyMerkleTreeInstruction}
 *
 * @param merkleTree
 * @param authority
 * @param depthSizePair
 * @returns
 */
export function createInitEmptyMerkleTreeIx(
    merkleTree: PublicKey,
    authority: PublicKey,
    depthSizePair: ValidDepthSizePair
): TransactionInstruction {
    return createInitEmptyMerkleTreeInstruction(
        {
            authority: authority,
            merkleTree,
            noop: SPL_NOOP_PROGRAM_ID,
        },
        depthSizePair
    );
}

/**
 * Helper function for {@link createReplaceLeafInstruction}
 * @param merkleTree
 * @param authority
 * @param proof
 * @param newLeaf
 * @returns
 */
export function createReplaceIx(
    merkleTree: PublicKey,
    authority: PublicKey,
    newLeaf: Buffer,
    proof: MerkleTreeProof
): TransactionInstruction {
    return addProof(
        createReplaceLeafInstruction(
            {
                authority: authority,
                merkleTree,
                noop: SPL_NOOP_PROGRAM_ID,
            },
            {
                index: proof.leafIndex,
                newLeaf: Array.from(newLeaf),
                previousLeaf: Array.from(proof.leaf),
                root: Array.from(proof.root),
            }
        ),
        proof.proof
    );
}

/**
 * Helper function for {@link createAppendInstruction}
 * @param merkleTree
 * @param authority
 * @param newLeaf
 * @returns
 */
export function createAppendIx(
    merkleTree: PublicKey,
    authority: PublicKey,
    newLeaf: Buffer | ArrayLike<number>
): TransactionInstruction {
    return createAppendInstruction(
        {
            authority: authority,
            merkleTree,
            noop: SPL_NOOP_PROGRAM_ID,
        },
        {
            leaf: Array.from(newLeaf),
        }
    );
}

/**
 * Helper function for {@link createTransferAuthorityIx}
 * @param merkleTree
 * @param authority
 * @param newAuthority
 * @returns
 */
export function createTransferAuthorityIx(
    merkleTree: PublicKey,
    authority: PublicKey,
    newAuthority: PublicKey
): TransactionInstruction {
    return createTransferAuthorityInstruction(
        {
            authority: authority,
            merkleTree,
        },
        {
            newAuthority,
        }
    );
}

/**
 * Helper function for {@link createVerifyLeafInstruction}
 * @param merkleTree
 * @param proof
 * @returns
 */
export function createVerifyLeafIx(merkleTree: PublicKey, proof: MerkleTreeProof): TransactionInstruction {
    return addProof(
        createVerifyLeafInstruction(
            {
                merkleTree,
            },
            {
                index: proof.leafIndex,
                leaf: Array.from(proof.leaf),
                root: Array.from(proof.root),
            }
        ),
        proof.proof
    );
}

/**
 * Helper function for creating the {@link ConcurrentMerkleTreeAccount}.
 * It is best to use this method to initialize a {@link ConcurrentMerkleTreeAccount}
 * because these accounts can be quite large, and over the limit for what you
 * can allocate via CPI.
 * @param connection
 * @param merkleTree
 * @param payer
 * @param depthSizePair
 * @param canopyDepth
 * @returns
 */
export async function createAllocTreeIx(
    connection: Connection,
    merkleTree: PublicKey,
    payer: PublicKey,
    depthSizePair: ValidDepthSizePair,
    canopyDepth: number
): Promise<TransactionInstruction> {
    const requiredSpace = getConcurrentMerkleTreeAccountSize(
        depthSizePair.maxDepth,
        depthSizePair.maxBufferSize,
        canopyDepth ?? 0
    );
    return SystemProgram.createAccount({
        fromPubkey: payer,
        lamports: await connection.getMinimumBalanceForRentExemption(requiredSpace),
        newAccountPubkey: merkleTree,
        programId: PROGRAM_ID,
        space: requiredSpace,
    });
}

/**
 * Helper function for {@link createCloseEmptyTreeInstruction}.
 * @param merkleTree
 * @param authority
 * @param recipient
 * @returns
 */
export function createCloseEmptyTreeIx(
    merkleTree: PublicKey,
    authority: PublicKey,
    recipient: PublicKey
): TransactionInstruction {
    return createCloseEmptyTreeInstruction({
        authority,
        merkleTree,
        recipient,
    });
}
