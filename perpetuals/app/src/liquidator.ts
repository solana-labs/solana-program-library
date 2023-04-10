//@ts-nocheck
import { PublicKey } from "@solana/web3.js";
import { PerpetualsClient } from "./client";
import { Command } from "commander";
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";

let client;

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function initClient(clusterUrl: string, adminKeyPath: string) {
  process.env["ANCHOR_WALLET"] = adminKeyPath;
  client = new PerpetualsClient(clusterUrl, adminKeyPath);
  client.log("Client Initialized");
}

async function processLiquidations(
  poolName: string,
  tokenMint: PublicKey,
  rewardReceivingAccount: PublicKey
) {
  // read all positions
  let positions = await client.getPoolTokenPositions(poolName, tokenMint);

  let undercollateralized = 0;
  let liquidated = 0;
  for (const position of positions) {
    let position_side =
      JSON.stringify(position.side) === JSON.stringify({ long: {} })
        ? "long"
        : "short";

    // check position state
    let state = await client.getLiquidationState(
      position.owner,
      poolName,
      tokenMint,
      position_side
    );

    if (state === 1) {
      // liquidate over-leveraged positions
      undercollateralized += 1;

      let userTokenAccount = (
        await getOrCreateAssociatedTokenAccount(
          client.provider.connection,
          client.admin,
          tokenMint,
          position.owner
        )
      ).address;

      try {
        await client.liquidate(
          position.owner,
          poolName,
          tokenMint,
          position_side,
          userTokenAccount,
          rewardReceivingAccount
        );
      } catch (err) {
        continue;
      }

      liquidated += 1;
    }
  }

  return [undercollateralized, liquidated];
}

async function run(poolName: string, tokenMint: PublicKey) {
  let errorDelay = 10000;
  let liquidationDelay = 5000;

  let rewardReceivingAccount = (
    await getOrCreateAssociatedTokenAccount(
      client.provider.connection,
      client.admin,
      tokenMint,
      client.admin.publicKey
    )
  ).address;

  // main loop
  while (true) {
    let perpetuals;
    try {
      perpetuals = await client.getPerpetuals();
    } catch (err) {
      console.error(err);
    }

    if (!perpetuals.permissions.allowClosePosition) {
      client.error(
        `Liquidations are not allowed at this time. Retrying in ${errorDelay} sec...`
      );
      await sleep(errorDelay);
      continue;
    }

    let [undercollateralized, liquidated] = await processLiquidations(
      poolName,
      tokenMint,
      rewardReceivingAccount
    );
    client.log(`Liquidated: ${liquidated} / ${undercollateralized}`);

    await sleep(liquidationDelay);
  }
}

(async function main() {
  const program = new Command();
  program
    .name("liquidator.ts")
    .description("Liquidator Bot for Solana Perpetuals Exchange Program")
    .version("0.1.0")
    .option(
      "-u, --url <string>",
      "URL for Solana's JSON RPC",
      "https://api.devnet.solana.com"
    )
    .requiredOption("-k, --keypair <path>", "Filepath to the admin keypair")
    .hook("preSubcommand", (thisCommand, subCommand) => {
      initClient(program.opts().url, program.opts().keypair);
      client.log(`Processing command '${thisCommand.args[0]}'`);
    })
    .hook("postAction", () => {
      client.log("Done");
    });

  program
    .command("run")
    .description("Run the bot")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .action(async (poolName, tokenMint) => {
      await run(poolName, new PublicKey(tokenMint));
    });

  await program.parseAsync(process.argv);

  if (!process.argv.slice(2).length) {
    program.outputHelp();
  }
})();
