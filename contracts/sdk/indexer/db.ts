import sqlite3 from "sqlite3";
import { open, Database, Statement } from "sqlite";
import { PathNode } from "../gummyroll";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { keccak_256 } from "js-sha3";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { NewLeafEvent } from "./indexer/bubblegum";
import { BN } from "@project-serum/anchor";
import { bignum } from "@metaplex-foundation/beet";
import {
  Creator,
  LeafSchema,
} from "../bubblegum/src/generated";
import { ChangeLogEvent } from "./indexer/gummyroll";
let fs = require("fs");

/**
 * Uses on-chain hash fn to hash together buffers
 */
export function hash(left: Buffer, right: Buffer): Buffer {
  return Buffer.from(keccak_256.digest(Buffer.concat([left, right])));
}

export class NFTDatabaseConnection {
  connection: Database<sqlite3.Database, sqlite3.Statement>;
  tree: Map<number, [number, string]>;
  emptyNodeCache: Map<number, Buffer>;

  constructor(connection: Database<sqlite3.Database, sqlite3.Statement>) {
    this.connection = connection;
    this.tree = new Map<number, [number, string]>();
    this.emptyNodeCache = new Map<number, Buffer>();
  }

  async beginTransaction() {
    return this.connection.run("BEGIN TRANSACTION");
  }

  async rollback() {
    return this.connection.run("ROLLBACK");
  }

  async commit() {
    return this.connection.run("COMMIT");
  }

  async upsertRowsFromBackfill(rows: Array<[PathNode, number, number]>) {
    this.connection.db.serialize(() => {
      this.connection.run("BEGIN TRANSACTION");
      for (const [node, seq, i] of rows) {
        this.connection.run(
          `
            INSERT INTO 
            merkle(node_idx, seq, level, hash)
            VALUES (?, ?, ?, ?)
          `,
          node.index,
          seq,
          i,
          node.node.toBase58()
        );
      }
      this.connection.run("COMMIT");
    });
  }

  async updateChangeLogs(changeLog: ChangeLogEvent, txId: string) {
    console.log("Update Change Log");
    if (changeLog.seq == 0) {
      return;
    }
    for (const [i, pathNode] of changeLog.path.entries()) {
      this.connection.run(
        `
          INSERT INTO 
          merkle(transaction_id, node_idx, seq, level, hash)
          VALUES (?, ?, ?, ?, ?)
        `,
        txId,
        pathNode.index,
        changeLog.seq,
        i,
        new PublicKey(pathNode.node).toBase58()
      );
    }
  }

  async updateLeafSchema(leafSchema: LeafSchema, leafHash: PublicKey, txId: string) {
    console.log("Update Leaf Schema");
    this.connection.run(
      `
        INSERT INTO
        leaf_schema(
          nonce,
          transaction_id,
          owner,
          delegate,
          data_hash,
          creator_hash,
          leaf_hash  
        )
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT (nonce)
        DO UPDATE SET 
          owner = excluded.owner,
          delegate = excluded.delegate,
          data_hash = excluded.data_hash,
          creator_hash = excluded.creator_hash,
          leaf_hash = excluded.leaf_hash
      `,
      (leafSchema.nonce.valueOf() as BN).toNumber(),
      txId,
      leafSchema.owner.toBase58(),
      leafSchema.delegate.toBase58(),
      bs58.encode(leafSchema.dataHash),
      bs58.encode(leafSchema.creatorHash),
      leafHash.toBase58()
    );
  }

  async updateNFTMetadata(newLeafEvent: NewLeafEvent, nonce: bignum) {
    console.log("Update NFT");
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
    this.connection.run(
      `
        INSERT INTO 
        nft(
          nonce,
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
        ON CONFLICT (nonce)
        DO UPDATE SET
          uri = excluded.uri,
          name = excluded.name,
          symbol = excluded.symbol,
          primary_sale_happened = excluded.primary_sale_happened,
          seller_fee_basis_points = excluded.seller_fee_basis_points,
          is_mutable = excluded.is_mutable
      `,
      (nonce as BN).toNumber(),
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

  async updateTree() {
    let res = await this.connection.all(
      `
        SELECT DISTINCT 
        node_idx, hash, level, max(seq) as seq
        FROM merkle
        GROUP BY node_idx
      `
    );
    for (const row of res) {
      this.tree.set(row.node_idx, [row.seq, row.hash]);
    }
    return res;
  }

  async getSequenceNumbers() {
    return new Set<number>(
      (
        await this.connection.all(
          `
            SELECT DISTINCT seq 
            FROM merkle
            ORDER by seq
          `
        )
      ).map((x) => x.seq)
    );
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

  async getProof(hash: Buffer, check: boolean = true): Promise<Proof | null> {
    let hashString = bs58.encode(hash);
    let res = await this.connection.all(
      `
        SELECT 
          DISTINCT m.node_idx as nodeIdx,
          l.data_hash as dataHash,
          l.creator_hash as creatorHash,
          l.nonce as nonce,
          l.owner as owner,
          l.delegate as delegate,
          max(m.seq) as seq
        FROM merkle m
        JOIN leaf_schema l
        ON m.hash = l.leaf_hash
        WHERE hash = ? and level = 0
        GROUP BY node_idx
      `,
      hashString
    );
    if (res.length == 1) {
      let data = res[0]
      return this.generateProof(
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
    nodeIdx: number,
    hash: Buffer,
    dataHash: string,
    creatorHash: string,
    nonce: number,
    owner: string,
    delegate: string,
    check: boolean = true
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
    let res = await this.connection.all(
      `
      SELECT DISTINCT node_idx, hash, level, max(seq) as seq
      FROM merkle where node_idx in (${nodes.join(",")})
      GROUP BY node_idx
      ORDER BY level
      `
    );
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
    if (check && !this.verifyProof(inferredProof)) {
      console.log("Proof is invalid");
      return null;
    }
    return inferredProof;
  }

  verifyProof(proof: Proof) {
    let node = bs58.decode(proof.leaf);
    let index = proof.index;
    for (const [i, pNode] of proof.proofNodes.entries()) {
      if ((index >> i) % 2 === 0) {
        node = hash(node, new PublicKey(pNode).toBuffer());
      } else {
        node = hash(new PublicKey(pNode).toBuffer(), node);
      }
    }
    const rehashed = new PublicKey(node).toString();
    const received = new PublicKey(proof.root).toString();
    return rehashed === received;
  }

  async getAssetsForOwner(owner: string) {
    let rawNftMetadata = await this.connection.all(
      `
      SELECT
        ls.nonce as nonce,
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
      ON ls.nonce = n.nonce
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

  if (create) {
    await db.run(
      `
        CREATE TABLE IF NOT EXISTS merkle (
          id INTEGER PRIMARY KEY,
          transaction_id TEXT,
          node_idx INT,
          seq INT,
          level INT,
          hash TEXT
        );
      `
    );

    await db.run(
      `
      CREATE TABLE IF NOT EXISTS nft (
        nonce BIGINT PRIMARY KEY,
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
    await db.run(
      `
      CREATE TABLE IF NOT EXISTS leaf_schema (
        nonce BIGINT PRIMARY KEY,
        transaction_id TEXT,
        owner TEXT,
        delegate TEXT,
        data_hash TEXT,
        creator_hash TEXT,
        leaf_hash TEXT
      );
      `
    );
    await db.run(
      `
      CREATE TABLE IF NOT EXISTS creators (
        nonce BIGINT,
        creator TEXT,
        share INT,
        verifed BOOLEAN 
      );
      `
    );
  }

  return new NFTDatabaseConnection(db);
}
