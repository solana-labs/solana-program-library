
import * as sqlite3 from "sqlite3";
import {
    open,
    Database
} from 'sqlite'
import {
    existsSync,
    rmSync,
    mkdirSync,
} from 'fs';
import { join } from 'path';
import { PublicKey } from '@solana/web3.js';
import { BN } from '@project-serum/anchor';
import { ChangeLogEventV1 } from '@solana/spl-account-compression';

import { DatabaseConfig, DEFAULT_DB_FILE_NAME } from "../config";
import { GapInfo } from "../../ingest/types/Gap";
import { TreeRow } from '../../merkle-tree'

export class Sqlite3 {
    config: DatabaseConfig;
    db!: Database;

    constructor(config: DatabaseConfig) {
        this.config = config;
    }

    async bootstrap() {
        if (!existsSync(this.config.tableDir)) {
            mkdirSync(this.config.tableDir);
        } else if (this.config.reset) {
            rmSync(this.config.tableDir, { recursive: true });
            if (existsSync(this.config.tableDir)) {
                throw Error("Failed to delete existing table dir as required by --reset flag. Please remove manually")
            }
        }

        const filename = join(this.config.tableDir, DEFAULT_DB_FILE_NAME);
        this.db = await open({
            filename,
            driver: sqlite3.Database,
        });

        // Allows concurrency in SQLITE
        await this.db.run("PRAGMA journal_mode = WAL;");


        // This creates the tables if needed
        this.db.db.serialize(() => {
            this.db.run("BEGIN TRANSACTION");
            this.db.run(
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
            this.db.run(
                `
                    CREATE INDEX IF NOT EXISTS sequence_number
                    ON merkle(seq)
                  `
            );
            this.db.run(
                `
                    CREATE INDEX IF NOT EXISTS nodes 
                    ON merkle(node_idx)
                `
            );
            // TODO(ngundotra): '--snapshot' flag this
            this.db.run(
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
            this.db.run("COMMIT");
        });
    }

    /**
     * Upsert change logs
     */
    async updateChangeLogs(
        changeLog: ChangeLogEventV1,
        transactionId: string,
        slot: number,
    ) {
        // Avoid inserting empty rows
        if (changeLog.seq.eq(new BN(0))) {
            return;
        }
        for (const [i, pathNode] of changeLog.path.entries()) {
            await this.db
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
                    transactionId,
                    slot,
                    changeLog.treeId.toBase58(),
                    pathNode.index,
                    changeLog.seq.toNumber(),
                    i,
                    new PublicKey(pathNode.node).toBase58()
                )
                .catch((e) => {
                    console.log("DB error on ChangeLog upsert", e);
                });
        }
    }

    /**
     * Returns rows of the merkle tree from the most recent changelogs (but not more recent than maxSeq)
     */
    async getTreeRows(treeId: PublicKey, maxSeq: number | null): Promise<TreeRow[]> {
        let res: any[];
        if (maxSeq) {
            res = await this.db.all(
                `
              SELECT DISTINCT 
              node_idx, hash, level, max(seq) as seq
              FROM merkle
              where tree_id = ? and seq <= ?
              GROUP BY node_idx
            `,
                treeId.toBase58(),
                maxSeq
            );
        } else {
            res = await this.db.all(
                `
              SELECT DISTINCT 
              node_idx, hash, level, max(seq) as seq
              FROM merkle
              where tree_id = ?
              GROUP BY node_idx
            `,
                treeId.toBase58()
            );
        }
        return res.map((row) => {
            return {
                nodeIndex: row.node_idx,
                hash: row.hash,
                level: row.level,
                seq: row.seq
            }
        })
    }

    async hasCMT(treeId: PublicKey): Promise<boolean> {
        const results = await this.db.all(
            `
                SELECT * from merkle
                where tree_id = ?
            `,
            treeId.toString()
        );
        console.log("this has not been approved by the FDA, consume at your own risk");
        return results.length > 0
    }

    /**
     * Returns the maximum sequence number recorded in this database
     */
    async getLatestTreeInfo(treeId: PublicKey): Promise<{
        seq: number,
        slot: number,
        transactionId: string
    }> {
        // TODO(ngundotra): will have to debug this
        let res = await this.db.get(
            `
            SELECT max(seq) as seq, slot, transaction_id
            FROM merkle
            GROUP BY slot, transaction_id
            LIMIT 1
            `,
            treeId.toBase58()
        );
        if (res) {
            return {
                seq: res.seq,
                slot: res.slot,
                transactionId: res.transaction_id
            };
        } else {
            throw Error("Unable to find treeId in the database")
        }
    }

    async getFirstKnownMissingSeq(treeId: PublicKey): Promise<number> {
        let gapIndex = await this.db.get(
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
            treeId.toBase58(),
            treeId.toBase58()
        );
        return gapIndex.seq;
    }

    async getTreeRowsForNodes(
        treeId: PublicKey,
        /** Tree-space indices */
        nodes: number[],
        maxSequenceNumber: number,
    ): Promise<TreeRow[]> {
        let res = await this.db.all(
            `
            SELECT DISTINCT 
            node_idx, hash, level, max(seq) as seq
            FROM merkle WHERE 
              node_idx in (${nodes.join(",")}) AND tree_id = ? AND seq <= ?
            GROUP BY node_idx
            ORDER BY level
            `,
            treeId.toBase58(),
            maxSequenceNumber
        );

        return res.map((row) => {
            return {
                nodeIndex: row.node_idx,
                hash: row.hash,
                level: row.level,
                seq: row.seq
            }
        })
    }

    /**
     * Returns gaps in the database
     */
    async getMissingData(treeId: PublicKey, minSeq: number = 0): Promise<GapInfo[]> {
        let gaps: Array<GapInfo> = [];
        let res = await this.db
            .all(
                `
                    SELECT DISTINCT seq, slot, transaction_id
                    FROM merkle
                    WHERE tree_id = ? and seq >= ?
                    order by seq
                `,
                treeId,
                minSeq
            )
            .catch((e) => {
                console.log("Failed to make query", e);
                return [];
            });

        // Find gaps by taking the different between slot numbers
        for (let i = 0; i < res.length - 1; ++i) {
            let [previousSeq, previousSlot, previousTransactionId] = [res[i].seq, res[i].slot, res[i].transaction_id];
            let [currentSeq, currentSlot, currentTransactionId] = [res[i + 1].seq, res[i + 1].slot, res[i + 1].transaction_id];
            if (currentSeq === previousSeq) {
                throw new Error(
                    `Error in DB, encountered identical sequence numbers with different slots: ${previousSlot} ${currentSlot}`
                );
            }
            if (currentSeq - previousSeq > 1) {
                gaps.push({ previousSeq, currentSeq, previousSlot, currentSlot, previousTransactionId, currentTransactionId });
            }
        }

        if (res.length > 0) {
            return gaps;
        }
        return gaps;
    }

}