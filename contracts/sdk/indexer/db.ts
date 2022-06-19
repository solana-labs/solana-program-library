import sqlite3 from "sqlite3";
import { open, Database, Statement } from "sqlite";
import { PathNode } from "../gummyroll";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { keccak_256 } from "js-sha3";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { LeafSchemaEvent, NewLeafEvent } from "./indexer/bubblegum";
import { BN } from "@project-serum/anchor";
import { bignum } from "@metaplex-foundation/beet";
import { Creator } from "../bubblegum/src/generated";
import { ChangeLogEvent } from "./indexer/gummyroll";
let fs = require("fs");

/**
 * Uses on-chain hash fn to hash together buffers
 */
export function hash(left: Buffer, right: Buffer): Buffer {
  return Buffer.from(keccak_256.digest(Buffer.concat([left, right])));
}

export type GapInfo = {
  prevSeq: number;
  currSeq: number;
  prevSlot: number;
  currSlot: number;
};
export class NFTDatabaseConnection {
  connection: Database<sqlite3.Database, sqlite3.Statement>;
  emptyNodeCache: Map<number, Buffer>;

  constructor(connection: Database<sqlite3.Database, sqlite3.Statement>) {
    this.connection = connection;
    this.emptyNodeCache = new Map<number, Buffer>();
  }

  async beginTransaction() {
    return await this.connection.run("BEGIN TRANSACTION");
  }

  async rollback() {
    return await this.connection.run("ROLLBACK");
  }

  async commit() {
    return await this.connection.run("COMMIT");
  }

  async updateChangeLogs(
    changeLog: ChangeLogEvent,
    txId: string,
    slot: number,
    treeId: string
  ) {
    if (changeLog.seq == 0) {
      return;
    }
    for (const [i, pathNode] of changeLog.path.entries()) {
      await this.connection
        .run(
          `
          INSERT INTO 
          merkle(transaction_id, slot, tree_id, node_idx, seq, level, hash)
          VALUES (?, ?, ?, ?, ?, ?, ?)
          ON CONFLICT (tree_id, seq, node_idx)
          DO UPDATE SET
            transaction_id = excluded.transaction_id,
            slot = excluded.slot,
            tree_id = excluded.tree_id,
            level = excluded.level,
            hash = excluded.hash
        `,
          txId,
          slot,
          treeId,
          pathNode.index,
          changeLog.seq,
          i,
          new PublicKey(pathNode.node).toBase58()
        )
        .catch((e) => {
          console.log("DB error on change log upsert", e);
        });
    }
  }

  async updateLeafSchema(
    leafSchemaRecord: LeafSchemaEvent,
    leafHash: PublicKey,
    txId: string,
    slot: number,
    sequenceNumber: number,
    treeId: string,
    compressed: boolean = true,
  ) {
    const leafSchema = leafSchemaRecord.schema.v1;
    await this.connection.run(
      `
        INSERT INTO
        leaf_schema(
          asset_id,
          nonce,
          tree_id,
          seq,
          transaction_id,
          slot,
          owner,
          delegate,
          data_hash,
          creator_hash,
          leaf_hash,
          compressed
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT (nonce, tree_id)
        DO UPDATE SET 
          asset_id = excluded.asset_id,
          seq = excluded.seq,
          transaction_id = excluded.transaction_id,
          owner = excluded.owner,
          delegate = excluded.delegate,
          data_hash = excluded.data_hash,
          creator_hash = excluded.creator_hash,
          leaf_hash = excluded.leaf_hash,
          compressed = excluded.compressed
      `,
      leafSchema.id.toBase58(),
      (leafSchema.nonce.valueOf() as BN).toNumber(),
      treeId,
      sequenceNumber,
      txId,
      slot,
      leafSchema.owner.toBase58(),
      leafSchema.delegate.toBase58(),
      bs58.encode(leafSchema.dataHash),
      bs58.encode(leafSchema.creatorHash),
      leafHash.toBase58(),
      compressed
    );
  }

  async updateNFTMetadata(
    newLeafEvent: NewLeafEvent,
    assetId: string
  ) {
    const uri = newLeafEvent.metadata.uri;
    const name = newLeafEvent.metadata.name;
    const symbol = newLeafEvent.metadata.symbol;
    const primarySaleHappened = newLeafEvent.metadata.primarySaleHappened;
    const sellerFeeBasisPoints = newLeafEvent.metadata.sellerFeeBasisPoints;
    const isMutable = newLeafEvent.metadata.isMutable;
    let creators: Array<Creator> = [];
    for (let i = 0; i < 5; ++i) {
      if (newLeafEvent.metadata.creators.length < i + 1) {
        creators.push({
          address: SystemProgram.programId,
          share: 0,
          verified: false,
        });
      } else {
        creators.push(newLeafEvent.metadata.creators[i]);
      }
    }
    await this.connection.run(
      `
        INSERT INTO 
        nft(
          asset_id,
          uri,
          name,
          symbol,
          primary_sale_happened,
          seller_fee_basis_points,
          is_mutable,
          creator0,
          share0,
          verified0,
          creator1,
          share1,
          verified1,
          creator2,
          share2,
          verified2,
          creator3,
          share3,
          verified3,
          creator4,
          share4,
          verified4
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT (asset_id)
        DO UPDATE SET
          uri = excluded.uri,
          name = excluded.name,
          symbol = excluded.symbol,
          primary_sale_happened = excluded.primary_sale_happened,
          seller_fee_basis_points = excluded.seller_fee_basis_points,
          is_mutable = excluded.is_mutable,
          creator0 = excluded.creator0,
          share0 = excluded.share0,
          verified0 = excluded.verified0,
          creator1 = excluded.creator1,
          share1 = excluded.share1,
          verified1 = excluded.verified1,
          creator2 = excluded.creator2,
          share2 = excluded.share2,
          verified2 = excluded.verified2,
          creator3 = excluded.creator3,
          share3 = excluded.share3,
          verified3 = excluded.verified3,
          creator4 = excluded.creator4,
          share4 = excluded.share4,
          verified4 = excluded.verified4
      `,
      assetId,
      uri,
      name,
      symbol,
      primarySaleHappened,
      sellerFeeBasisPoints,
      isMutable,
      creators[0].address,
      creators[0].share,
      creators[0].verified,
      creators[1].address,
      creators[1].share,
      creators[1].verified,
      creators[2].address,
      creators[2].share,
      creators[2].verified,
      creators[3].address,
      creators[3].share,
      creators[3].verified,
      creators[4].address,
      creators[4].share,
      creators[4].verified
    );
  }

  emptyNode(level: number): Buffer {
    if (this.emptyNodeCache.has(level)) {
      return this.emptyNodeCache.get(level);
    }
    if (level == 0) {
      return Buffer.alloc(32);
    }
    let result = hash(this.emptyNode(level - 1), this.emptyNode(level - 1));
    this.emptyNodeCache.set(level, result);
    return result;
  }

  async getTree(treeId: string, maxSeq: number | null) {
    let res;
    if (maxSeq) {
      res = await this.connection.all(
        `
          SELECT DISTINCT 
          node_idx, hash, level, max(seq) as seq
          FROM merkle
          where tree_id = ? and seq <= ?
          GROUP BY node_idx
        `,
        treeId,
        maxSeq
      );
    } else {
      res = await this.connection.all(
        `
          SELECT DISTINCT 
          node_idx, hash, level, max(seq) as seq
          FROM merkle
          where tree_id = ?
          GROUP BY node_idx
        `,
        treeId
      );
    }
    return res;
  }

  async getMissingData(minSeq: number, treeId: string) {
    let gaps: Array<GapInfo> = [];
    let res = await this.connection
      .all(
        `
        SELECT DISTINCT seq, slot
        FROM merkle
        where tree_id = ? and seq >= ?
        order by seq
      `,
        treeId,
        minSeq
      )
      .catch((e) => {
        console.log("Failed to make query", e);
        return [gaps, null, null];
      });
    for (let i = 0; i < res.length - 1; ++i) {
      let [prevSeq, prevSlot] = [res[i].seq, res[i].slot];
      let [currSeq, currSlot] = [res[i + 1].seq, res[i + 1].slot];
      if (currSeq === prevSeq) {
        throw new Error(
          `Error in DB, encountered identical sequence numbers with different slots: ${prevSlot} ${currSlot}`
        );
      }
      if (currSeq - prevSeq > 1) {
        gaps.push({ prevSeq, currSeq, prevSlot, currSlot });
      }
    }
    if (res.length > 0) {
      return [gaps, res[res.length - 1].seq, res[res.length - 1].slot];
    }
    return [gaps, null, null];
  }

  async getTrees() {
    let res = await this.connection
      .all(
        `
        SELECT DISTINCT tree_id, max(level) as depth
        FROM merkle
        GROUP BY tree_id
      `
      )
      .catch((e) => {
        console.log("Failed to query table", e);
        return [];
      });

    return res.map((x) => {
      return [x.tree_id, x.depth];
    });
  }

  async getAllLeaves() {
    let leaves = await this.connection.all(
      `
        SELECT DISTINCT node_idx, hash, max(seq) as seq
        FROM merkle
        WHERE level = 0
        GROUP BY node_idx
        ORDER BY node_idx
      `
    );
    let leafHashes = new Set<string>();
    if (leaves.length > 0) {
      for (const l of leaves) {
        leafHashes.add(l.hash);
      }
    }
    return leafHashes;
  }

  async getLeafIndices(): Promise<Array<[number, Buffer]>> {
    let leaves = await this.connection.all(
      `
        SELECT DISTINCT node_idx, hash, max(seq) as seq
        FROM merkle
        WHERE level = 0
        GROUP BY node_idx
        ORDER BY node_idx
      `
    );
    let leafIdxs = [];
    if (leaves.length > 0) {
      for (const l of leaves) {
        leafIdxs.push([l.node_idx, bs58.decode(l.hash)]);
      }
    }
    return leafIdxs;
  }

  async getMaxSeq(treeId: string) {
    let res = await this.connection.get(
      `
        SELECT max(seq) as seq
        FROM merkle 
        WHERE tree_id = ?
      `,
      treeId
    );
    if (res) {
      return res.seq;
    } else {
      return null;
    }
  }
  async getInferredProof(
    hash: Buffer,
    treeId: string,
    check: boolean = true
  ): Promise<Proof | null> {
    let latestSeq = await this.getMaxSeq(treeId);
    if (!latestSeq) {
      return null;
    }
    let gapIndex = await this.connection.get(
      `
        SELECT 
          m0.seq as seq
        FROM merkle m0
        WHERE NOT EXISTS (
          SELECT NULL
          FROM merkle m1
          WHERE m1.seq = m0.seq + 1 AND m1.tree_id = ?
        ) AND tree_id = ?
        ORDER BY m0.seq
        LIMIT 1
      `,
      treeId,
      treeId
    );
    if (gapIndex && gapIndex.seq < latestSeq) {
      return await this.inferProofWithKnownGap(
        hash,
        treeId,
        gapIndex.seq,
        check
      );
    } else {
      return await this.getProof(hash, treeId, check);
    }
  }

  async inferProofWithKnownGap(
    hash: Buffer,
    treeId: string,
    seq: number,
    check: boolean = true
  ) {
    let hashString = bs58.encode(hash);
    let depth = await this.getDepth(treeId);
    if (!depth) {
      return null;
    }

    let res = await this.connection.get(
      `
        SELECT 
          data_hash as dataHash,
          creator_hash as creatorHash,
          nonce as nonce,
          owner as owner,
          delegate as delegate
        FROM leaf_schema
        WHERE leaf_hash = ? and tree_id = ?
      `,
      hashString,
      treeId
    );
    if (res) {
      return this.generateProof(
        treeId,
        (1 << depth) + res.nonce,
        hash,
        res.dataHash,
        res.creatorHash,
        res.nonce,
        res.owner,
        res.delegate,
        check,
        seq
      );
    } else {
      return null;
    }
  }

  async getDepth(treeId: string) {
    let res = await this.connection.get(
      `
        SELECT max(level) as depth
        FROM merkle 
        WHERE tree_id = ?
      `,
      treeId
    );
    if (res) {
      return res.depth;
    } else {
      return null;
    }
  }

  async getProof(
    hash: Buffer,
    treeId: string,
    check: boolean = true
  ): Promise<Proof | null> {
    let hashString = bs58.encode(hash);
    let res = await this.connection.all(
      `
        SELECT 
          m.node_idx as nodeIdx,
          l.data_hash as dataHash,
          l.creator_hash as creatorHash,
          l.nonce as nonce,
          l.owner as owner,
          l.delegate as delegate
        FROM merkle m
        JOIN leaf_schema l
        ON m.hash = l.leaf_hash and m.tree_id = l.tree_id and m.seq = l.seq
        WHERE hash = ? and m.tree_id = ? and level = 0
      `,
      hashString,
      treeId
    );
    if (res.length == 1) {
      let data = res[0];
      return this.generateProof(
        treeId,
        data.nodeIdx,
        hash,
        data.dataHash,
        data.creatorHash,
        data.nonce,
        data.owner,
        data.delegate,
        check
      );
    } else {
      return null;
    }
  }

  async generateProof(
    treeId: string,
    nodeIdx: number,
    hash: Buffer,
    dataHash: string,
    creatorHash: string,
    nonce: number,
    owner: string,
    delegate: string,
    check: boolean = true,
    maxSequenceNumber: number | null = null
  ): Promise<Proof | null> {
    let nodes = [];
    let n = nodeIdx;
    while (n > 1) {
      if (n % 2 == 0) {
        nodes.push(n + 1);
      } else {
        nodes.push(n - 1);
      }
      n >>= 1;
    }
    nodes.push(1);
    let res;
    if (maxSequenceNumber) {
      res = await this.connection.all(
        `
        SELECT DISTINCT node_idx, hash, level, max(seq) as seq
        FROM merkle WHERE 
          node_idx in (${nodes.join(",")}) AND tree_id = ? AND seq <= ?
        GROUP BY node_idx
        ORDER BY level
        `,
        treeId,
        maxSequenceNumber
      );
    } else {
      res = await this.connection.all(
        `
        SELECT DISTINCT node_idx, hash, level, max(seq) as seq
        FROM merkle WHERE 
          node_idx in (${nodes.join(",")}) AND tree_id = ?
        GROUP BY node_idx
        ORDER BY level
        `,
        treeId
      );
    }

    if (res.length < 1) {
      return null;
    }
    let root = res.pop();
    if (root.node_idx != 1) {
      return null;
    }
    let proof = [];
    for (let i = 0; i < root.level; i++) {
      proof.push(bs58.encode(this.emptyNode(i)));
    }
    for (const node of res) {
      proof[node.level] = node.hash;
    }
    let leafIdx = nodeIdx - (1 << root.level);
    let inferredProof = {
      leaf: bs58.encode(hash),
      root: root.hash,
      proofNodes: proof,
      index: leafIdx,
      nonce,
      dataHash: dataHash,
      creatorHash: creatorHash,
      owner: owner,
      delegate: delegate,
    };
    if (maxSequenceNumber) {
      // If this parameter is set, we directly attempt to infer the root value
      inferredProof.root = bs58.encode(this.generateRoot(inferredProof));
    }
    if (check && !this.verifyProof(inferredProof)) {
      console.log("Proof is invalid");
      return null;
    }
    return inferredProof;
  }

  generateRoot(proof: Proof) {
    let node = bs58.decode(proof.leaf);
    let index = proof.index;
    for (const [i, pNode] of proof.proofNodes.entries()) {
      if ((index >> i) % 2 === 0) {
        node = hash(node, new PublicKey(pNode).toBuffer());
      } else {
        node = hash(new PublicKey(pNode).toBuffer(), node);
      }
    }
    return node;
  }

  verifyProof(proof: Proof) {
    let node = this.generateRoot(proof);
    const rehashed = new PublicKey(node).toString();
    const received = new PublicKey(proof.root).toString();
    return rehashed === received;
  }

  async getAssetsForOwner(owner: string) {
    let rawNftMetadata = await this.connection.all(
      `
      SELECT
        ls.tree_id as treeId,
        ls.nonce as nonce,
        n.asset_id as assetId,
        n.uri as uri,
        n.name as name,
        n.symbol as symbol,
        n.seller_fee_basis_points as sellerFeeBasisPoints,
        ls.owner as owner,
        ls.delegate as delegate,
        ls.leaf_hash as leafHash,
        n.creator0 as creator0,
        n.share0 as share0,
        n.verified0 as verified0,
        n.creator1 as creator1,
        n.share1 as share1,
        n.verified1 as verified1,
        n.creator2 as creator2,
        n.share2 as share2,
        n.verified2 as verified2,
        n.creator3 as creator3,
        n.share3 as share3,
        n.verified3 as verified3,
        n.creator4 as creator4,
        n.share4 as share4,
        n.verified4 as verified4
      FROM leaf_schema ls
      JOIN nft n
      ON ls.asset_id = n.asset_id
      WHERE owner = ?
      `,
      owner
    );
    let assets = [];
    for (const metadata of rawNftMetadata) {
      let creators: Creator[] = [];
      if (metadata.creator0 !== SystemProgram.programId.toBase58()) {
        creators.push({
          address: metadata.creator0,
          share: metadata.share0,
          verified: metadata.verified0,
        });
      }
      if (metadata.creator1 !== SystemProgram.programId.toBase58()) {
        creators.push({
          address: metadata.creator1,
          share: metadata.share1,
          verified: metadata.verified1,
        });
      }
      if (metadata.creator2 !== SystemProgram.programId.toBase58()) {
        creators.push({
          address: metadata.creator2,
          share: metadata.share2,
          verified: metadata.verified2,
        });
      }
      if (metadata.creator3 !== SystemProgram.programId.toBase58()) {
        creators.push({
          address: metadata.creator3,
          share: metadata.share3,
          verified: metadata.verified3,
        });
      }
      if (metadata.creator4 !== SystemProgram.programId.toBase58()) {
        creators.push({
          address: metadata.creator4,
          share: metadata.share4,
          verified: metadata.verified4,
        });
      }
      assets.push({
        nonce: metadata.nonce,
        treeId: metadata.treeId,
        assetId: metadata.assetId,
        uri: metadata.uri,
        name: metadata.name,
        symbol: metadata.symbol,
        sellerFeeBasisPoints: metadata.sellerFeeBasisPoints,
        owner: metadata.owner,
        delegate: metadata.delegate,
        leafHash: metadata.leafHash,
        creators: creators,
      });
    }
    return assets;
  }
}

export type Proof = {
  dataHash: string;
  creatorHash: string;
  owner: string;
  delegate: string;
  nonce: number;
  root: string;
  leaf: string;
  proofNodes: string[];
  index: number;
};

// this is a top-level await
export async function bootstrap(
  create: boolean = true
): Promise<NFTDatabaseConnection> {
  // open the database
  const dir = "db";
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir);
  }
  const db = await open({
    filename: `${dir}/merkle.db`,
    driver: sqlite3.Database,
  });

  // Allows concurrency in SQLITE
  await db.run("PRAGMA journal_mode = WAL;");

  if (create) {
    db.db.serialize(() => {
      db.run("BEGIN TRANSACTION");
      db.run(
        `
          CREATE TABLE IF NOT EXISTS merkle (
            tree_id TEXT,
            transaction_id TEXT,
            slot INT,
            node_idx INT,
            seq INT,
            level INT,
            hash TEXT,
            PRIMARY KEY (tree_id, seq, node_idx) 
          );
        `
      );
      db.run(
        `
          CREATE INDEX IF NOT EXISTS sequence_number
          ON merkle(seq)
        `
      );
      db.run(
        `
          CREATE INDEX IF NOT EXISTS nodes 
          ON merkle(node_idx)
        `
      );
      db.run(
        `
        CREATE TABLE IF NOT EXISTS nft (
          asset_id TEXT PRIMARY KEY,
          name TEXT,
          symbol TEXT,
          uri TEXT,
          seller_fee_basis_points INT, 
          primary_sale_happened BOOLEAN, 
          is_mutable BOOLEAN,
          creator0 TEXT,
          share0 INT,
          verified0 BOOLEAN,
          creator1 TEXT,
          share1 INT,
          verified1 BOOLEAN,
          creator2 TEXT,
          share2 INT,
          verified2 BOOLEAN,
          creator3 TEXT,
          share3 INT,
          verified3 BOOLEAN,
          creator4 TEXT,
          share4 INT,
          verified4 BOOLEAN
        );
        `
      );
      db.run(
        `
        CREATE TABLE IF NOT EXISTS leaf_schema (
          asset_id TEXT,
          tree_id TEXT,
          nonce BIGINT,
          seq INT,
          slot INT,
          transaction_id TEXT,
          owner TEXT,
          delegate TEXT,
          data_hash TEXT,
          creator_hash TEXT,
          leaf_hash TEXT,
          compressed BOOLEAN,
          PRIMARY KEY (tree_id, nonce)
        );
        `
      );
      db.run(
        `
          CREATE TABLE IF NOT EXISTS merkle_snapshot (
            max_seq INT,
            tree_id TEXT,
            transaction_id TEXT,
            node_idx INT,
            seq INT,
            level INT,
            hash TEXT
          );
        `
      );
      db.run(
        `
          CREATE INDEX IF NOT EXISTS assets
          ON leaf_schema(asset_id)
        `
      );
      db.run("COMMIT");
    });
  }

  return new NFTDatabaseConnection(db);
}
