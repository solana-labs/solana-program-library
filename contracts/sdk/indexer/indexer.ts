import { Keypair } from "@solana/web3.js";
import { Connection } from "@solana/web3.js";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "../bubblegum/src/generated";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from "../gummyroll/index";
import * as anchor from "@project-serum/anchor";
import { Bubblegum } from "../../target/types/bubblegum";
import { Gummyroll } from "../../target/types/gummyroll";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { loadProgram, handleLogs, handleLogsAtomic } from "./indexer/utils";
import { bootstrap } from "./db";
import { fetchAndPlugGaps, validateTree } from "./backfiller";

// const url = "http://api.internal.mainnet-beta.solana.com";
const url = "http://127.0.0.1:8899";
let Bubblegum: anchor.Program<Bubblegum>;
let Gummyroll: anchor.Program<Gummyroll>;

async function main() {
  const endpoint = url;
  const connection = new Connection(endpoint, "confirmed");
  const payer = Keypair.generate();
  const provider = new anchor.Provider(connection, new NodeWallet(payer), {
    commitment: "confirmed",
  });
  let db = await bootstrap();
  console.log("Finished bootstrapping DB");
  Gummyroll = loadProgram(
    provider,
    GUMMYROLL_PROGRAM_ID,
    "target/idl/gummyroll.json"
  ) as anchor.Program<Gummyroll>;
  Bubblegum = loadProgram(
    provider,
    BUBBLEGUM_PROGRAM_ID,
    "target/idl/bubblegum.json"
  ) as anchor.Program<Bubblegum>;
  console.log("loaded programs...");
  let subscriptionId = connection.onLogs(
    BUBBLEGUM_PROGRAM_ID,
    (logs, ctx) => handleLogsAtomic(db, logs, ctx, { Gummyroll, Bubblegum }),
    "confirmed"
  );
  while (true) {
    try {
      const trees = await db.getTrees();
      for (const [treeId, depth] of trees) {
        console.log("Scanning for gaps");
        await fetchAndPlugGaps(connection, db, 0, treeId, {
          Gummyroll,
          Bubblegum,
        });
        console.log("Validation:");
        console.log(
          `    Off-chain tree ${treeId} is consistent: ${await validateTree(
            db,
            depth,
            treeId,
            0,
          )}`
        );
        console.log("Moving to next tree");
      }
    } catch (e) {
      console.log("ERROR");
      console.log(e);
      continue;
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
}

main();
