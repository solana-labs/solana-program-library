import { Connection, Keypair, PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { SPL_NOOP_PROGRAM_ID } from "../constants";
import { getConcurrentMerkleTreeAccountSize } from '../accounts';
import {
  createReplaceLeafInstruction,
  createAppendInstruction,
  createTransferAuthorityInstruction,
  createVerifyLeafInstruction,
  PROGRAM_ID,
  createInitEmptyMerkleTreeInstruction,
  createCloseEmptyTreeInstruction
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
  merkleTree: PublicKey,
  authority: PublicKey,
  maxDepth: number,
  maxBufferSize: number
): TransactionInstruction {
  return createInitEmptyMerkleTreeInstruction(
    {
      merkleTree,
      authority: authority,
      noop: SPL_NOOP_PROGRAM_ID,
    },
    {
      maxBufferSize,
      maxDepth
    }
  );
}

export function createReplaceIx(
  merkleTree: PublicKey,
  authority: PublicKey,
  treeRoot: Buffer,
  previousLeaf: Buffer,
  newLeaf: Buffer,
  index: number,
  proof: Buffer[]
): TransactionInstruction {
  return addProof(createReplaceLeafInstruction(
    {
      merkleTree,
      authority: authority,
      noop: SPL_NOOP_PROGRAM_ID,
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
  merkleTree: PublicKey,
  authority: PublicKey,
  newLeaf: Buffer | ArrayLike<number>,
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
  )
}

export function createTransferAuthorityIx(
  merkleTree: PublicKey,
  authority: PublicKey,
  newAuthority: PublicKey,
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
  merkleTree: PublicKey,
  payer: PublicKey,
  maxBufferSize: number,
  maxDepth: number,
  canopyDepth: number,
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

export function createCloseEmptyTreeIx(
  merkleTree: PublicKey,
  authority: PublicKey,
  recipient: PublicKey,
): TransactionInstruction {
  return createCloseEmptyTreeInstruction(
    {
      merkleTree,
      authority,
      recipient
    },
  )
}