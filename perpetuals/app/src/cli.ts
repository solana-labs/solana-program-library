/// Command-line interface for basic admin functions

import { BN } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";
import { PerpetualsClient, PositionSide } from "./client";
import { Command } from "commander";

let client;

function initClient(clusterUrl: string, adminKeyPath: string) {
  process.env["ANCHOR_WALLET"] = adminKeyPath;
  client = new PerpetualsClient(clusterUrl, adminKeyPath);
  client.log("Client Initialized");
}

async function init(adminSigners: PublicKey[], minSignatures: number) {
  // to be loaded from config file
  let perpetualsConfig = {
    minSignatures: minSignatures,
    allowSwap: true,
    allowAddLiquidity: true,
    allowRemoveLiquidity: true,
    allowOpenPosition: true,
    allowClosePosition: true,
    allowPnlWithdrawal: true,
    allowCollateralWithdrawal: true,
    allowSizeChange: true,
  };
  client.init(adminSigners, perpetualsConfig);
}

async function setAuthority(adminSigners: PublicKey[], minSignatures: number) {
  client.setAdminSigners(adminSigners, minSignatures);
}

async function getMultisig() {
  client.prettyPrint(await client.getMultisig());
}

async function getPerpetuals() {
  client.prettyPrint(await client.getPerpetuals());
}

async function addPool(poolName: string) {
  client.addPool(poolName);
}

async function getPool(poolName: string) {
  client.prettyPrint(await client.getPool(poolName));
}

async function getPools() {
  client.prettyPrint(await client.getPools());
}

async function removePool(poolName: string) {
  client.removePool(poolName);
}

async function addCustody(
  poolName: string,
  tokenMint: PublicKey,
  tokenOracle: PublicKey,
  isStable: boolean
) {
  // to be loaded from config file
  let oracleConfig = {
    maxPriceError: new BN(10000),
    maxPriceAgeSec: 60,
    oracleType: { pyth: {} },
    oracleAccount: tokenOracle,
  };
  let pricingConfig = {
    useEma: true,
    useUnrealizedPnlInAum: true,
    tradeSpreadLong: new BN(100),
    tradeSpreadShort: new BN(100),
    swapSpread: new BN(200),
    minInitialLeverage: new BN(10000),
    maxInitialLeverage: new BN(1000000),
    maxLeverage: new BN(1000000),
    maxPayoffMult: new BN(10000),
    maxUtilization: new BN(10000),
    maxPositionLockedUsd: new BN(1000000000),
    maxTotalLockedUsd: new BN(1000000000),
  };
  let permissions = {
    allowSwap: true,
    allowAddLiquidity: true,
    allowRemoveLiquidity: true,
    allowOpenPosition: true,
    allowClosePosition: true,
    allowPnlWithdrawal: true,
    allowCollateralWithdrawal: true,
    allowSizeChange: true,
  };
  let fees = {
    mode: { linear: {} },
    ratioMult: new BN(20000),
    utilizationMult: new BN(20000),
    swapIn: new BN(100),
    swapOut: new BN(100),
    stableSwapIn: new BN(100),
    stableSwapOut: new BN(100),
    addLiquidity: new BN(100),
    removeLiquidity: new BN(100),
    openPosition: new BN(100),
    closePosition: new BN(100),
    liquidation: new BN(100),
    protocolShare: new BN(10),
  };
  let borrowRate = {
    baseRate: new BN(0),
    slope1: new BN(80000),
    slope2: new BN(120000),
    optimalUtilization: new BN(800000000),
  };

  let pool = await client.getPool(poolName);
  pool.ratios.push({
    target: new BN(5000),
    min: new BN(10),
    max: new BN(10000),
  });
  let ratios = client.adjustTokenRatios(pool.ratios);

  client.addCustody(
    poolName,
    tokenMint,
    isStable,
    oracleConfig,
    pricingConfig,
    permissions,
    fees,
    borrowRate,
    ratios
  );
}

async function getCustody(poolName: string, tokenMint: PublicKey) {
  client.prettyPrint(await client.getCustody(poolName, tokenMint));
}

async function getCustodies(poolName: string) {
  client.prettyPrint(await client.getCustodies(poolName));
}

async function removeCustody(poolName: string, tokenMint: PublicKey) {
  let pool = await client.getPool(poolName);
  pool.ratios.pop();
  let ratios = client.adjustTokenRatios(pool.ratios);

  client.removeCustody(poolName, tokenMint, ratios);
}

async function upgradeCustody(poolName: string, tokenMint: PublicKey) {
  client.upgradeCustody(poolName, tokenMint);
}

async function getUserPosition(
  wallet: PublicKey,
  poolName: string,
  tokenMint: PublicKey,
  side: PositionSide
) {
  client.prettyPrint(
    await client.getUserPosition(wallet, poolName, tokenMint, side)
  );
}

async function getUserPositions(wallet: PublicKey) {
  client.prettyPrint(await client.getUserPositions(wallet));
}

async function getPoolTokenPositions(poolName: string, tokenMint: PublicKey) {
  client.prettyPrint(await client.getPoolTokenPositions(poolName, tokenMint));
}

async function getAllPositions() {
  client.prettyPrint(await client.getAllPositions());
}

async function getAddLiquidityAmountAndFee(
  poolName: string,
  tokenMint: PublicKey,
  amount: BN
) {
  client.prettyPrint(
    await client.getAddLiquidityAmountAndFee(poolName, tokenMint, amount)
  );
}

async function getRemoveLiquidityAmountAndFee(
  poolName: string,
  tokenMint: PublicKey,
  lpAmount: BN
) {
  client.prettyPrint(
    await client.getRemoveLiquidityAmountAndFee(poolName, tokenMint, lpAmount)
  );
}

async function getEntryPriceAndFee(
  poolName: string,
  tokenMint: PublicKey,
  collateral: BN,
  size: BN,
  side: PositionSide
) {
  client.prettyPrint(
    await client.getEntryPriceAndFee(
      poolName,
      tokenMint,
      collateral,
      size,
      side
    )
  );
}

async function getExitPriceAndFee(
  wallet: PublicKey,
  poolName: string,
  tokenMint: PublicKey,
  side: PositionSide
) {
  client.prettyPrint(
    await client.getExitPriceAndFee(wallet, poolName, tokenMint, side)
  );
}

async function getOraclePrice(
  poolName: string,
  tokenMint: PublicKey,
  useEma: boolean
) {
  client.prettyPrint(await client.getOraclePrice(poolName, tokenMint, useEma));
}

async function getLiquidationPrice(
  wallet: PublicKey,
  poolName: string,
  tokenMint: PublicKey,
  side: PositionSide,
  addCollateral: BN,
  removeCollateral: BN
) {
  client.prettyPrint(
    await client.getLiquidationPrice(
      wallet,
      poolName,
      tokenMint,
      side,
      addCollateral,
      removeCollateral
    )
  );
}

async function getLiquidationState(
  wallet: PublicKey,
  poolName: string,
  tokenMint: PublicKey,
  side: PositionSide
) {
  client.prettyPrint(
    await client.getLiquidationState(wallet, poolName, tokenMint, side)
  );
}

async function getPnl(
  wallet: PublicKey,
  poolName: string,
  tokenMint: PublicKey,
  side: PositionSide
) {
  client.prettyPrint(await client.getPnl(wallet, poolName, tokenMint, side));
}

async function getSwapAmountAndFees(
  poolName: string,
  tokenMintIn: PublicKey,
  tokenMintOut: PublicKey,
  amountIn: BN
) {
  client.prettyPrint(
    await client.getSwapAmountAndFees(
      poolName,
      tokenMintIn,
      tokenMintOut,
      amountIn
    )
  );
}

async function getAum(poolName: string) {
  client.prettyPrint(await client.getAum(poolName));
}

(async function main() {
  const program = new Command();
  program
    .name("cli.ts")
    .description("CLI to Solana Perpetuals Exchange Program")
    .version("0.1.0")
    .option(
      "-u, --url <string>",
      "URL for Solana's JSON RPC",
      "https://api.devnet.solana.com"
    )
    .requiredOption("-k, --keypair <path>", "Filepath to the admin keypair")
    .hook("preSubcommand", (thisCommand, subCommand) => {
      if (!program.opts().keypair) {
        throw Error("required option '-k, --keypair <path>' not specified");
      }
      initClient(program.opts().url, program.opts().keypair);
      client.log(`Processing command '${thisCommand.args[0]}'`);
    })
    .hook("postAction", () => {
      client.log("Done");
    });

  program
    .command("init")
    .description("Initialize the on-chain program")
    .requiredOption("-m, --min-signatures <int>", "Minimum signatures")
    .argument("<pubkey...>", "Admin public keys")
    .action(async (args, options) => {
      await init(
        args.map((x) => new PublicKey(x)),
        options.minSignatures
      );
    });

  program
    .command("set-authority")
    .description("Set protocol admins")
    .requiredOption("-m, --min-signatures <int>", "Minimum signatures")
    .argument("<pubkey...>", "Admin public keys")
    .action(async (args, options) => {
      await setAuthority(
        args.map((x) => new PublicKey(x)),
        options.minSignatures
      );
    });

  program
    .command("get-multisig")
    .description("Print multisig state")
    .action(async () => {
      await getMultisig();
    });

  program
    .command("get-perpetuals")
    .description("Print perpetuals global state")
    .action(async () => {
      await getPerpetuals();
    });

  program
    .command("add-pool")
    .description("Create a new pool")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      await addPool(poolName);
    });

  program
    .command("get-pool")
    .description("Print metadata for the pool")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      await getPool(poolName);
    });

  program
    .command("get-pools")
    .description("Print metadata for all pools")
    .action(async () => {
      await getPools();
    });

  program
    .command("remove-pool")
    .description("Remove the pool")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      await removePool(poolName);
    });

  program
    .command("add-custody")
    .description("Add a new token custody to the pool")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<pubkey>", "Token oracle account")
    .option("-s, --stablecoin", "Custody is for a stablecoin")
    .action(async (poolName, tokenMint, tokenOracle, options) => {
      await addCustody(
        poolName,
        new PublicKey(tokenMint),
        new PublicKey(tokenOracle),
        options.stablecoin
      );
    });

  program
    .command("get-custody")
    .description("Print metadata for the token custody")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .action(async (poolName, tokenMint) => {
      await getCustody(poolName, new PublicKey(tokenMint));
    });

  program
    .command("get-custodies")
    .description("Print metadata for all custodies")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      await getCustodies(poolName);
    });

  program
    .command("remove-custody")
    .description("Remove the token custody from the pool")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .action(async (poolName, tokenMint) => {
      await removeCustody(poolName, new PublicKey(tokenMint));
    });

  program
    .command("upgrade-custody")
    .description("Upgrade deprecated custody to the new version")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .action(async (poolName, tokenMint, options) => {
      await upgradeCustody(poolName, new PublicKey(tokenMint));
    });

  program
    .command("get-user-position")
    .description("Print user position metadata")
    .argument("<pubkey>", "User wallet")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Position side (long / short)")
    .action(async (wallet, poolName, tokenMint, side) => {
      await getUserPosition(
        new PublicKey(wallet),
        poolName,
        new PublicKey(tokenMint),
        side
      );
    });

  program
    .command("get-user-positions")
    .description("Print all user positions")
    .argument("<pubkey>", "User wallet")
    .action(async (wallet) => {
      await getUserPositions(new PublicKey(wallet));
    });

  program
    .command("get-pool-token-positions")
    .description("Print positions in the token")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .action(async (poolName, tokenMint) => {
      await getPoolTokenPositions(poolName, new PublicKey(tokenMint));
    });

  program
    .command("get-all-positions")
    .description("Print all open positions")
    .action(async () => {
      await getAllPositions();
    });

  program
    .command("get-add-liquidity-amount-and-fee")
    .description("Compute LP amount returned and fee for add liquidity")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .requiredOption("-a, --amount <bigint>", "Token amount")
    .action(async (poolName, tokenMint, options) => {
      await getAddLiquidityAmountAndFee(
        poolName,
        new PublicKey(tokenMint),
        new BN(options.amount)
      );
    });

  program
    .command("get-remove-liquidity-amount-and-fee")
    .description("Compute token amount returned and fee for remove liquidity")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .requiredOption("-a, --amount <bigint>", "LP token amount")
    .action(async (poolName, tokenMint, options) => {
      await getRemoveLiquidityAmountAndFee(
        poolName,
        new PublicKey(tokenMint),
        new BN(options.amount)
      );
    });

  program
    .command("get-entry-price-and-fee")
    .description("Compute price and fee to open a position")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Position side (long / short)")
    .requiredOption("-c, --collateral <bigint>", "Collateral")
    .requiredOption("-s, --size <bigint>", "Size")
    .action(async (poolName, tokenMint, side, options) => {
      await getEntryPriceAndFee(
        poolName,
        new PublicKey(tokenMint),
        new BN(options.collateral),
        new BN(options.size),
        side
      );
    });

  program
    .command("get-exit-price-and-fee")
    .description("Compute price and fee to close the position")
    .argument("<pubkey>", "User wallet")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Position side (long / short)")
    .action(async (wallet, poolName, tokenMint, side) => {
      await getExitPriceAndFee(
        new PublicKey(wallet),
        poolName,
        new PublicKey(tokenMint),
        side
      );
    });

  program
    .command("get-oracle-price")
    .description("Read oracle price for the token")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .option("-e, --ema", "Return EMA price")
    .action(async (poolName, tokenMint, options) => {
      await getOraclePrice(poolName, new PublicKey(tokenMint), options.ema);
    });

  program
    .command("get-liquidation-price")
    .description("Compute liquidation price for the position")
    .argument("<pubkey>", "User wallet")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Position side (long / short)")
    .option("-a, --add-collateral <bigint>", "Collateral to add")
    .option("-r, --remove-collateral <bigint>", "Collateral to remove")
    .action(async (wallet, poolName, tokenMint, side, options) => {
      await getLiquidationPrice(
        new PublicKey(wallet),
        poolName,
        new PublicKey(tokenMint),
        side,
        new BN(options.addCollateral),
        new BN(options.removeCollateral)
      );
    });

  program
    .command("get-liquidation-state")
    .description("Get liquidation state of the position")
    .argument("<pubkey>", "User wallet")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Position side (long / short)")
    .action(async (wallet, poolName, tokenMint, side) => {
      await getLiquidationState(
        new PublicKey(wallet),
        poolName,
        new PublicKey(tokenMint),
        side
      );
    });

  program
    .command("get-pnl")
    .description("Compute PnL of the position")
    .argument("<pubkey>", "User wallet")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint")
    .argument("<string>", "Position side (long / short)")
    .action(async (wallet, poolName, tokenMint, side) => {
      await getPnl(
        new PublicKey(wallet),
        poolName,
        new PublicKey(tokenMint),
        side
      );
    });

  program
    .command("get-swap-amount-and-fees")
    .description("Compute amount out and fees for the swap")
    .argument("<string>", "Pool name")
    .argument("<pubkey>", "Token mint in")
    .argument("<pubkey>", "Token mint out")
    .requiredOption("-i, --amount-in <bigint>", "Token amount to be swapped")
    .action(async (poolName, tokenMintIn, tokenMintOut, options) => {
      await getSwapAmountAndFees(
        poolName,
        new PublicKey(tokenMintIn),
        new PublicKey(tokenMintOut),
        new BN(options.amountIn)
      );
    });

  program
    .command("get-aum")
    .description("Get assets under management")
    .argument("<string>", "Pool name")
    .action(async (poolName) => {
      await getAum(poolName);
    });

  await program.parseAsync(process.argv);

  if (!process.argv.slice(2).length) {
    program.outputHelp();
  }
})();
