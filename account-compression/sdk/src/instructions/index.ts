import {
  Connection,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from '@solana/web3.js';

import { SPL_NOOP_PROGRAM_ID, ValidDepthSizePair } from '../constants';
import { getConcurrentMerkleTreeAccountSize } from '../accounts';
import {
  createReplaceLeafInstruction,
  createAppendInstruction,
  createTransferAuthorityInstruction,
  createVerifyLeafInstruction,
  PROGRAM_ID,
  createInitEmptyMerkleTreeInstruction,
  createCloseEmptyTreeInstruction,
} from '../generated';
import { MerkleTreeProof } from '../merkle-tree';

/**
 * Helper function that adds proof nodes to a TransactionInstruction
 * by adding extra keys to the transaction
 */
export function addProof(
  instruction: TransactionInstruction,
  nodeProof: Buffer[]
): TransactionInstruction {
  instruction.keys = instruction.keys.concat(
    nodeProof.map((node) => {
      return {
        pubkey: new PublicKey(node),
        isSigner: false,
        isWritable: false,
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
      merkleTree,
      authority: authority,
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
        merkleTree,
        authority: authority,
        noop: SPL_NOOP_PROGRAM_ID,
      },
      {
        root: Array.from(proof.root),
        previousLeaf: Array.from(proof.leaf),
        newLeaf: Array.from(newLeaf),
        index: proof.leafIndex,
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
      merkleTree,
      authority: authority,
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
      merkleTree,
      authority: authority,
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
export function createVerifyLeafIx(
  merkleTree: PublicKey,
  proof: MerkleTreeProof
): TransactionInstruction {
  return addProof(
    createVerifyLeafInstruction(
      {
        merkleTree,
      },
      {
        root: Array.from(proof.root),
        leaf: Array.from(proof.leaf),
        index: proof.leafIndex,
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
    newAccountPubkey: merkleTree,
    lamports: await connection.getMinimumBalanceForRentExemption(requiredSpace),
    space: requiredSpace,
    programId: PROGRAM_ID,
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
    merkleTree,
    authority,
    recipient,
  });
}
