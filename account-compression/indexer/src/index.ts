import { PublicKey } from '@solana/web3.js';
import {
    program
} from 'commander';
import { backfill } from './actions';

program
    .name("SPL Account Compression Indexer")
    .description("CLI to running your own indexer")
    .version("0.1.0")

program
    .command("backfill")
    .description("Backfill a programId's data into a table")
    .argument("<treeId>", "tree address to backfill")
    .argument("<tableName>", "postgres table to write data into")
    .argument("<rpcUrl>", "url of rpc to query")
    .action(async (treeId, tableName, rpcUrl) => {
        console.log({ treeId, tableName, rpcUrl });

        backfill({ treeId: new PublicKey(treeId), tableName, rpcUrl })
            .then(() => {
                console.log("Backfill completed")
            })
        // .catch((e: Error) => {
        //     console.log("Backfill failed with error:", e);
        //     console.log(e.stack);
        // })
    })

program.parse();