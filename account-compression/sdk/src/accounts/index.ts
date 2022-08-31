import { PublicKey, Connection } from "@solana/web3.js";
import * as borsh from "borsh";
import * as BN from 'bn.js';
import { assert } from "chai";
import { readPublicKey } from "../utils";

/**
 * Manually create a model for MerkleRoll in order to deserialize correctly
 */
export type SplConcurrentMerkleTree = {
  header: ConcurrentMerkleTreeHeader;
  tree: ConcurrentMerkleTree;
};

type ConcurrentMerkleTreeHeader = {
  accountType: number,
  _padding: number[]
  maxDepth: number; // u32
  maxBufferSize: number; // u32
  authority: PublicKey;
  creationSlot: BN;
};

type ConcurrentMerkleTree = {
  sequenceNumber: BN; // u64
  activeIndex: number; // u64
  bufferSize: number; // u64
  changeLogs: ChangeLog[];
  rightMostPath: Path;
};

export type PathNode = {
  node: PublicKey;
  index: number;
};

type ChangeLog = {
  root: PublicKey;
  pathNodes: PublicKey[];
  index: number; // u32
  _padding: number; // u32
};

type Path = {
  leaf: PublicKey;
  proof: PublicKey[];
  index: number;
  _padding: number;
};

export function deserializeConcurrentMerkleTree(buffer: Buffer): SplConcurrentMerkleTree {
  let reader = new borsh.BinaryReader(buffer);

  let header: ConcurrentMerkleTreeHeader = {
    accountType: reader.readU8(),
    _padding: Array.from(reader.readFixedArray(7)),
    maxBufferSize: reader.readU32(),
    maxDepth: reader.readU32(),
    authority: readPublicKey(reader),
    creationSlot: reader.readU64(),
  };

  let sequenceNumber = reader.readU64();
  let activeIndex = reader.readU64().toNumber();
  let bufferSize = reader.readU64().toNumber();

  let changeLogs: ChangeLog[] = [];
  for (let i = 0; i < header.maxBufferSize; i++) {
    let root = readPublicKey(reader);

    let pathNodes: PublicKey[] = [];
    for (let j = 0; j < header.maxDepth; j++) {
      pathNodes.push(readPublicKey(reader));
    }
    changeLogs.push({
      pathNodes,
      root,
      index: reader.readU32(),
      _padding: reader.readU32(),
    });
  }

  // Decode Right-Most Path
  let leaf = readPublicKey(reader);
  let proof: PublicKey[] = [];
  for (let j = 0; j < header.maxDepth; j++) {
    proof.push(readPublicKey(reader));
  }
  const rightMostPath = {
    proof,
    leaf,
    index: reader.readU32(),
    _padding: reader.readU32(),
  };

  const tree = {
    sequenceNumber,
    activeIndex,
    bufferSize,
    changeLogs,
    rightMostPath,
  };

  if (
    getConcurrentMerkleTreeSize(header.maxDepth, header.maxBufferSize) !=
    reader.offset
  ) {

    throw new Error(
      "Failed to process whole buffer when deserializing Merkle Account Data"
    );
  }
  return { header, tree };
}

export function getConcurrentMerkleTreeSize(
  maxDepth: number,
  maxBufferSize: number,
  canopyDepth?: number
): number {
  let headerSize = 8 + 8 + 8 + 32;
  let changeLogSize = (maxDepth * 32 + 32 + 4 + 4) * maxBufferSize;
  let rightMostPathSize = maxDepth * 32 + 32 + 4 + 4;
  let merkleRollSize = 8 + 8 + 8 + changeLogSize + rightMostPathSize;
  let canopySize = 0;
  if (canopyDepth) {
    canopySize = ((1 << canopyDepth + 1) - 2) * 32
  }
  return headerSize + merkleRollSize + canopySize;
}

export async function assertCMTProperties(
  connection: Connection,
  expectedMaxDepth: number,
  expectedMaxBufferSize: number,
  expectedAuthority: PublicKey,
  expectedRoot: Buffer,
  onChainCMTKey: PublicKey
) {
  const onChainCMT = await getConcurrentMerkleTree(connection, onChainCMTKey);

  assert(
    getCMTMaxDepth(onChainCMT) === expectedMaxDepth,
    `Max depth does not match ${getCMTMaxDepth(onChainCMT)}, expected ${expectedMaxDepth}`
  );
  assert(
    getCMTMaxBufferSize(onChainCMT) === expectedMaxBufferSize,
    `Max buffer size does not match ${getCMTMaxBufferSize(onChainCMT)}, expected ${expectedMaxBufferSize}`
  );
  assert(
    getCMTAuthority(onChainCMT).equals(expectedAuthority),
    "Failed to write auth pubkey"
  );
  assert(
    getCMTCurrentRoot(onChainCMT).equals(expectedRoot),
    "On chain root does not match root passed in instruction"
  );
}

export function getCMTMaxBufferSize(onChainCMT: SplConcurrentMerkleTree): number {
  return onChainCMT.header.maxBufferSize;
}

export function getCMTMaxDepth(onChainCMT: SplConcurrentMerkleTree): number {
  return onChainCMT.header.maxDepth;
}

export function getCMTBufferSize(onChainCMT: SplConcurrentMerkleTree): number {
  return onChainCMT.tree.bufferSize;
}

export function getCMTCurrentRoot(onChainCMT: SplConcurrentMerkleTree): Buffer {
  return onChainCMT.tree.changeLogs[getCMTActiveIndex(onChainCMT)].root.toBuffer();
}

export function getCMTActiveIndex(onChainCMT: SplConcurrentMerkleTree): number {
  return onChainCMT.tree.activeIndex
}

export function getCMTAuthority(onChainCMT: SplConcurrentMerkleTree): PublicKey {
  return onChainCMT.header.authority;
}

export async function getConcurrentMerkleTree(connection: Connection, onChainCMTKey: PublicKey): Promise<SplConcurrentMerkleTree> {
  const onChainCMTAccount = await connection.getAccountInfo(onChainCMTKey);
  if (!onChainCMTAccount) {
    throw new Error("CMT account data unexpectedly null!");
  }
  return deserializeConcurrentMerkleTree(onChainCMTAccount.data);
}