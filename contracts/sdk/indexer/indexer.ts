import { Keypair } from "@solana/web3.js";
import { Connection, Context, Logs } from "@solana/web3.js";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "../bubblegum/src/generated";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from "../gummyroll/index";
import * as anchor from "@project-serum/anchor";
import { Bubblegum } from "../../target/types/bubblegum";
import { Gummyroll } from "../../target/types/gummyroll";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { loadProgram, ParsedLog, parseLogs } from "./indexer/utils";
import { parseBubblegum } from "./indexer/bubblegum";
import { bootstrap, NFTDatabaseConnection } from "./db";

const MAX_DEPTH = 20;
const MAX_SIZE = 1024;
const localhostUrl = "http://127.0.0.1:8899";
let Bubblegum: anchor.Program<Bubblegum>;
let Gummyroll: anchor.Program<Gummyroll>;

function indexParsedLog(
  db: NFTDatabaseConnection,
  txId: string,
  parsedLog: ParsedLog | string
) {
  if (typeof parsedLog === "string") {
    return;
  }
  if (parsedLog.programId.equals(BUBBLEGUM_PROGRAM_ID)) {
    return parseBubblegum(
      db,
      parsedLog,
      { Bubblegum, Gummyroll },
      { txId: txId }
    );
  } else {
    for (const log of parsedLog.logs) {
      indexParsedLog(db, txId, log);
    }
  }
}

async function handleLogs(
  db: NFTDatabaseConnection,
  logs: Logs,
  _context: Context
) {
  if (logs.err) {
    return;
  }
  const parsedLogs = parseLogs(logs.logs);
  if (parsedLogs.length == 0) {
    return;
  }
  db.connection.db.serialize(() => {
    db.beginTransaction();
    for (const parsedLog of parsedLogs) {
      indexParsedLog(db, logs.signature, parsedLog);
    }
    console.log("Done executing queries");
    db.commit();
    console.log("Committed");
  });
}

async function main() {
  const endpoint = localhostUrl;
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
    async (logs, ctx) => await handleLogs(db, logs, ctx)
  );
}

main();
