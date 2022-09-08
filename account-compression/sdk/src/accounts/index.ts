import type { PublicKey, Connection } from "@solana/web3.js";
import * as borsh from "borsh";
import * as BN from 'bn.js';
import * as beet from '@metaplex-foundation/beet';
import * as beetSolana from '@metaplex-foundation/beet-solana';

/**
 * These are all the fields needed to deserialize the solana account
 * that the ConcurrentMerkleTree is stored in
 */
export type ConcurrentMerkleTreeAccount = {
  header: ConcurrentMerkleTreeHeader;
  tree: ConcurrentMerkleTree;
  canopy: Canopy,
};

type ConcurrentMerkleTreeHeader = {
  accountType: number;
  _padding: number[];
  maxBufferSize: number; // u32
  maxDepth: number; // u32
  authority: PublicKey;
  creationSlot: BN;
};

const concurrentMerkleTreeHeaderBeet = new beet.BeetArgsStruct<ConcurrentMerkleTreeHeader>(
  [
    ['accountType', beet.u8],
    ['_padding', beet.uniformFixedSizeArray(beet.u8, 7)],
    ['maxBufferSize', beet.u32],
    ['maxDepth', beet.u32],
    ['authority', beetSolana.publicKey],
    ['creationSlot', beet.u64],
  ],
  'ConcurrentMerkleTreeHeader'
);

type ChangeLog = {
  root: PublicKey,
  pathNodes: PublicKey[];
  index: number; // u32
  _padding: number; // u32
};

const changeLogBeetFactory = (maxDepth: number) => {
  return new beet.BeetArgsStruct<ChangeLog>(
    [
      ['root', beetSolana.publicKey],
      ['pathNodes', beet.uniformFixedSizeArray(beetSolana.publicKey, maxDepth)],
      ['index', beet.u32],
      ["_padding", beet.u32],
    ],
    'ChangeLog'
  )
}

type Path = {
  proof: PublicKey[];
  leaf: PublicKey;
  index: number; // u32
  _padding: number; // u32
};

const pathBeetFactory = (maxDepth: number) => {
  return new beet.BeetArgsStruct<Path>(
    [
      ['proof', beet.uniformFixedSizeArray(beetSolana.publicKey, maxDepth)],
      ['leaf', beetSolana.publicKey],
      ['index', beet.u32],
      ["_padding", beet.u32],
    ],
    'Path'
  )
}

type ConcurrentMerkleTree = {
  sequenceNumber: beet.bignum; // u64
  activeIndex: beet.bignum; // u64
  bufferSize: beet.bignum; // u64
  changeLogs: ChangeLog[];
  rightMostPath: Path;
};

export const concurrentMerkleTreeBeetFactory = (maxDepth: number, maxBufferSize: number) => {
  return new beet.BeetArgsStruct<ConcurrentMerkleTree>(
    [
      ['sequenceNumber', beet.u64],
      ['activeIndex', beet.u64],
      ['bufferSize', beet.u64],
      ['changeLogs', beet.uniformFixedSizeArray(changeLogBeetFactory(maxDepth), maxBufferSize)],
      ['rightMostPath', pathBeetFactory(maxDepth)],
    ],
    'ConcurrentMerkleTree'
  );
}

export type PathNode = {
  node: PublicKey;
  index: number;
};

type Canopy = {
  canopyBytes: number[];
}

const canopyBeetFactory = (canopyDepth: number) => {
  return new beet.BeetArgsStruct<Canopy>(
    [
      ['canopyBytes', beet.uniformFixedSizeArray(beet.u8, Math.max(((1 << canopyDepth + 1) - 2) * 32, 0))],
    ],
    'Canopy'
  );
}

function getCanopyDepth(canopyByteLength: number): number {
  if (canopyByteLength === 0) {
    return 0;
  }
  return Math.log2(canopyByteLength / 32 + 2) - 1
}

export function deserializeConcurrentMerkleTree(buffer: Buffer): ConcurrentMerkleTreeAccount {
  let offset = 0;
  const [header, offsetIncr] = concurrentMerkleTreeHeaderBeet.deserialize(buffer);
  offset = offsetIncr;

  const [tree, offsetIncr2] = concurrentMerkleTreeBeetFactory(header.maxDepth, header.maxBufferSize).deserialize(buffer, offset);
  offset = offsetIncr2;

  const canopyDepth = getCanopyDepth(buffer.byteLength - offset);
  let canopy: Canopy = {
    canopyBytes: []
  }
  if (canopyDepth !== 0) {
    const [deserializedCanopy, offsetIncr3] = canopyBeetFactory(canopyDepth).deserialize(buffer, offset);
    canopy = deserializedCanopy;
    offset = offsetIncr3;
  }

  if (buffer.byteLength !== offset) {
    throw new Error(
      "Failed to process whole buffer when deserializing Merkle Account Data"
    );
  }
  return { header, tree, canopy };
}

export function getConcurrentMerkleTreeAccountSize(
  maxDepth: number,
  maxBufferSize: number,
  canopyDepth?: number
): number {
  return concurrentMerkleTreeHeaderBeet.byteSize +
    concurrentMerkleTreeBeetFactory(maxDepth, maxBufferSize).byteSize +
    (canopyDepth ? canopyBeetFactory(canopyDepth).byteSize : 0);
}

export function getCMTMaxBufferSize(onChainCMT: ConcurrentMerkleTreeAccount): number {
  return onChainCMT.header.maxBufferSize;
}

export function getCMTMaxDepth(onChainCMT: ConcurrentMerkleTreeAccount): number {
  return onChainCMT.header.maxDepth;
}

export function getCMTBufferSize(onChainCMT: ConcurrentMerkleTreeAccount): number {
  return new BN.BN(onChainCMT.tree.bufferSize).toNumber();
}

export function getCMTCurrentRoot(onChainCMT: ConcurrentMerkleTreeAccount): Buffer {
  return onChainCMT.tree.changeLogs[getCMTActiveIndex(onChainCMT)].root.toBuffer();
}

export function getCMTActiveIndex(onChainCMT: ConcurrentMerkleTreeAccount): number {
  return new BN.BN(onChainCMT.tree.activeIndex).toNumber();
}

export function getCMTAuthority(onChainCMT: ConcurrentMerkleTreeAccount): PublicKey {
  return onChainCMT.header.authority;
}

export async function getConcurrentMerkleTree(connection: Connection, onChainCMTKey: PublicKey): Promise<ConcurrentMerkleTreeAccount> {
  const onChainCMTAccount = await connection.getAccountInfo(onChainCMTKey);
  if (!onChainCMTAccount) {
    throw new Error("CMT account data unexpectedly null!");
  }
  return deserializeConcurrentMerkleTree(onChainCMTAccount.data);
}