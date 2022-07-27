import { Keypair, PublicKey } from "@solana/web3.js";
import { Connection } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { Bubblegum } from "../../target/types/bubblegum";
import { Gummyroll } from "../../target/types/gummyroll";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { loadPrograms } from "./indexer/utils";
import { bootstrap } from "./db";
import { backfillTreeHistory, fillGapsTx, validateTree } from "./backfiller";

// const url = "http://api.explorer.mainnet-beta.solana.com";
const url = "http://127.0.0.1:8899";
let Bubblegum: anchor.Program<Bubblegum>;
let Gummyroll: anchor.Program<Gummyroll>;

async function main() {
    const treeId = process.argv[2];
    const endpoint = url;
    const connection = new Connection(endpoint, "confirmed");
    const payer = Keypair.generate();
    const provider = new anchor.AnchorProvider(connection, new NodeWallet(payer), {
        commitment: "confirmed",
    });
    let db = await bootstrap();
    console.log("Finished bootstrapping DB");

    const parserState = loadPrograms(provider);
    console.log("loaded programs...");

    // Fill gaps
    console.log("Filling in gaps for tree:", treeId);
    let { maxSeq, maxSeqSlot } = await fillGapsTx(connection, db, parserState, treeId);

    // Backfill to on-chain state, now with a complete db
    console.log(`Starting from slot!: ${maxSeqSlot} `);
    maxSeq = await backfillTreeHistory(connection, db, parserState, treeId, maxSeq, maxSeqSlot);

    // Validate
    console.log("Max SEQUENCE: ", maxSeq);
    const depth = await db.getDepth(treeId);
    console.log(`Tree ${treeId} has ${depth}`);
    console.log("Validating")
    console.log(
        `    Off - chain tree ${treeId} is consistent: ${await validateTree(
            db,
            depth,
            treeId,
            maxSeq,
        )
        } `
    );
}

main();
