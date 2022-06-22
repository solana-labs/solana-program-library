import { PublicKey, Connection } from "@solana/web3.js";
import * as borsh from "borsh";
import { BN } from "@project-serum/anchor";
import { assert } from "chai";
import { readPublicKey } from '../../utils';

/**
 * Manually create a model for MerkleRoll in order to deserialize correctly
 */
export class OnChainMerkleRoll {
  header: MerkleRollHeader;
  roll: MerkleRoll;

  constructor(header: MerkleRollHeader, roll: MerkleRoll) {
    this.header = header;
    this.roll = roll;
  }

  getChangeLogsWithNodeIndex(): PathNode[][] {
    const mask = this.header.maxBufferSize - 1;
    let pathNodeList = [];
    for (let j = 0; j < this.roll.bufferSize; j++) {
      let pathNodes = [];
      let idx = (this.roll.activeIndex - j) & mask;
      let changeLog = this.roll.changeLogs[idx];
      let pathLen = changeLog.pathNodes.length;
      for (const [lvl, key] of changeLog.pathNodes.entries()) {
        let nodeIdx = (1 << (pathLen - lvl)) + (changeLog.index >> lvl);
        pathNodes.push({
          node: key,
          index: nodeIdx,
        });
      }
      pathNodes.push({
        node: changeLog.root,
        index: 1,
      });
      pathNodeList.push(pathNodes);
    }
    return pathNodeList;
  }
}

type MerkleRollHeader = {
  maxDepth: number; // u32
  maxBufferSize: number; // u32
  authority: PublicKey;
  appendAuthority: PublicKey;
  creationSlot: BN;
};

type MerkleRoll = {
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

export function decodeMerkleRoll(buffer: Buffer): OnChainMerkleRoll {
  let reader = new borsh.BinaryReader(buffer);

  let header: MerkleRollHeader = {
    maxBufferSize: reader.readU32(),
    maxDepth: reader.readU32(),
    authority: readPublicKey(reader),
    appendAuthority: readPublicKey(reader),
    creationSlot: reader.readU64(),
  };

  // Decode MerkleRoll
  let sequenceNumber = reader.readU64();
  let activeIndex = reader.readU64().toNumber();
  let bufferSize = reader.readU64().toNumber();

  // Decode ChangeLogs
  let changeLogs = [];
  for (let i = 0; i < header.maxBufferSize; i++) {
    let root = readPublicKey(reader);

    let pathNodes = [];
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
  let proof = [];
  for (let j = 0; j < header.maxDepth; j++) {
    proof.push(readPublicKey(reader));
  }
  const rightMostPath = {
    proof,
    leaf,
    index: reader.readU32(),
    _padding: reader.readU32(),
  };

  const roll = {
    sequenceNumber,
    activeIndex,
    bufferSize,
    changeLogs,
    rightMostPath,
  };

  if (
    getMerkleRollAccountSize(header.maxDepth, header.maxBufferSize) !=
    reader.offset
  ) {
    throw new Error(
      "Failed to process whole buffer when deserializing Merkle Account Data"
    );
  }
  return new OnChainMerkleRoll(header, roll);
}

export function getMerkleRollAccountSize(
  maxDepth: number,
  maxBufferSize: number,
  canopyDepth?: number
): number {
  let headerSize = 8 + 32 + 32;
  let changeLogSize = (maxDepth * 32 + 32 + 4 + 4) * maxBufferSize;
  let rightMostPathSize = maxDepth * 32 + 32 + 4 + 4;
  let merkleRollSize = 8 + 8 + 16 + changeLogSize + rightMostPathSize;
  let canopySize = 0;
  if (canopyDepth) {
    canopySize = ((1 << canopyDepth + 1) - 2) * 32
  }
  return merkleRollSize + headerSize + canopySize;
}

export async function assertOnChainMerkleRollProperties(
  connection: Connection,
  expectedMaxDepth: number,
  expectedMaxBufferSize: number,
  expectedAuthority: PublicKey,
  expectedRoot: PublicKey,
  merkleRollPubkey: PublicKey
) {
  const merkleRoll = await connection.getAccountInfo(merkleRollPubkey);
  const merkleRollAcct = decodeMerkleRoll(merkleRoll.data);

  assert(
    merkleRollAcct.header.maxDepth === expectedMaxDepth,
    `Max depth does not match ${merkleRollAcct.header.maxDepth}, expected ${expectedMaxDepth}`
  );
  assert(
    merkleRollAcct.header.maxBufferSize === expectedMaxBufferSize,
    `Max buffer size does not match ${merkleRollAcct.header.maxBufferSize}, expected ${expectedMaxBufferSize}`
  );

  assert(
    merkleRollAcct.header.authority.equals(expectedAuthority),
    "Failed to write auth pubkey"
  );

  assert(
    merkleRollAcct.roll.changeLogs[0].root.equals(expectedRoot),
    "On chain root does not match root passed in instruction"
  );
}
