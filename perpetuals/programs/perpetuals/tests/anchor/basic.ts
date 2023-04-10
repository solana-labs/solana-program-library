import * as anchor from "@project-serum/anchor";
import { TestClient } from "./test_client";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import * as spl from "@solana/spl-token";
import { expect, assert } from "chai";
import { BN } from "bn.js";

describe("perpetuals", () => {
  let tc = new TestClient();
  tc.printErrors = true;
  let oracleConfig;
  let pricing;
  let permissions;
  let fees;
  let borrowRate;
  let ratios;
  let isStable;
  let perpetualsExpected;
  let multisigExpected;
  let tokenExpected;
  let positionExpected;

  it("init", async () => {
    await tc.initFixture();
    await tc.init();

    let err = await tc.ensureFails(tc.init());
    assert(err.logs[3].includes("already in use"));

    perpetualsExpected = {
      permissions: {
        allowSwap: true,
        allowAddLiquidity: true,
        allowRemoveLiquidity: true,
        allowOpenPosition: true,
        allowClosePosition: true,
        allowPnlWithdrawal: true,
        allowCollateralWithdrawal: true,
        allowSizeChange: true,
      },
      pools: [],
      transferAuthorityBump: tc.authority.bump,
      perpetualsBump: tc.perpetuals.bump,
      inceptionTime: new BN(0),
    };

    multisigExpected = {
      numSigners: 2,
      numSigned: 0,
      minSignatures: 2,
      instructionAccountsLen: 0,
      instructionDataLen: 0,
      instructionHash: new anchor.BN(0),
      signers: [
        tc.admins[0].publicKey,
        tc.admins[1].publicKey,
        PublicKey.default,
        PublicKey.default,
        PublicKey.default,
        PublicKey.default,
      ],
      signed: [false, false, false, false, false, false],
      bump: tc.multisig.bump,
    };

    let multisig = await tc.program.account.multisig.fetch(
      tc.multisig.publicKey
    );
    expect(JSON.stringify(multisig)).to.equal(JSON.stringify(multisigExpected));

    let perpetuals = await tc.program.account.perpetuals.fetch(
      tc.perpetuals.publicKey
    );
    expect(JSON.stringify(perpetuals)).to.equal(
      JSON.stringify(perpetualsExpected)
    );
  });

  it("setAdminSigners", async () => {
    await tc.setAdminSigners(1);

    let multisig = await tc.program.account.multisig.fetch(
      tc.multisig.publicKey
    );
    multisigExpected.minSignatures = 1;
    expect(JSON.stringify(multisig)).to.equal(JSON.stringify(multisigExpected));
  });

  it("setPermissions", async () => {
    perpetualsExpected.permissions = {
      allowSwap: true,
      allowAddLiquidity: true,
      allowRemoveLiquidity: true,
      allowOpenPosition: true,
      allowClosePosition: true,
      allowPnlWithdrawal: true,
      allowCollateralWithdrawal: true,
      allowSizeChange: true,
    };
    await tc.setPermissions(perpetualsExpected.permissions);

    let perpetuals = await tc.program.account.perpetuals.fetch(
      tc.perpetuals.publicKey
    );
    expect(JSON.stringify(perpetuals)).to.equal(
      JSON.stringify(perpetualsExpected)
    );
  });

  it("addAndRemovePool", async () => {
    await tc.addPool("test pool");

    let pool = await tc.program.account.pool.fetch(tc.pool.publicKey);
    let poolExpected = {
      name: "test pool",
      custodies: [],
      ratios: [],
      aumUsd: new BN(0),
      bump: tc.pool.bump,
      lpTokenBump: pool.lpTokenBump,
      inceptionTime: new BN(0),
    };
    expect(JSON.stringify(pool)).to.equal(JSON.stringify(poolExpected));

    await tc.removePool();
    tc.ensureFails(tc.program.account.pool.fetch(tc.pool.publicKey));

    await tc.addPool("test pool");
  });

  it("addAndRemoveCustody", async () => {
    oracleConfig = {
      maxPriceError: new BN(10000),
      maxPriceAgeSec: 60,
      oracleType: { test: {} },
      oracleAccount: tc.custodies[0].oracleAccount,
    };
    pricing = {
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
    permissions = {
      allowSwap: true,
      allowAddLiquidity: true,
      allowRemoveLiquidity: true,
      allowOpenPosition: true,
      allowClosePosition: true,
      allowPnlWithdrawal: true,
      allowCollateralWithdrawal: true,
      allowSizeChange: true,
    };
    fees = {
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
    borrowRate = {
      baseRate: new BN(0),
      slope1: new BN(80000),
      slope2: new BN(120000),
      optimalUtilization: new BN(800000000),
    };
    ratios = [
      {
        target: new BN(5000),
        min: new BN(10),
        max: new BN(10000),
      },
      {
        target: new BN(5000),
        min: new BN(10),
        max: new BN(10000),
      },
    ];
    let ratios1 = [
      {
        target: new BN(10000),
        min: new BN(10),
        max: new BN(10000),
      },
    ];
    isStable = false;
    await tc.addCustody(
      tc.custodies[0],
      isStable,
      oracleConfig,
      pricing,
      permissions,
      fees,
      borrowRate,
      ratios1
    );

    let token = await tc.program.account.custody.fetch(tc.custodies[0].custody);
    tokenExpected = {
      pool: tc.pool.publicKey,
      mint: tc.custodies[0].mint.publicKey,
      tokenAccount: tc.custodies[0].tokenAccount,
      decimals: 9,
      isStable,
      oracle: {
        oracleAccount: tc.custodies[0].oracleAccount,
        oracleType: { test: {} },
        maxPriceError: "10000",
        maxPriceAgeSec: 60,
      },
      pricing: {
        useEma: true,
        useUnrealizedPnlInAum: true,
        tradeSpreadLong: "100",
        tradeSpreadShort: "100",
        swapSpread: "200",
        minInitialLeverage: "10000",
        maxInitialLeverage: "1000000",
        maxLeverage: "1000000",
        maxPayoffMult: "10000",
        maxUtilization: "10000",
        maxPositionLockedUsd: "1000000000",
        maxTotalLockedUsd: "1000000000",
      },
      permissions: {
        allowSwap: true,
        allowAddLiquidity: true,
        allowRemoveLiquidity: true,
        allowOpenPosition: true,
        allowClosePosition: true,
        allowPnlWithdrawal: true,
        allowCollateralWithdrawal: true,
        allowSizeChange: true,
      },
      fees: {
        mode: { linear: {} },
        ratioMult: "20000",
        utilizationMult: "20000",
        swapIn: "100",
        swapOut: "100",
        stableSwapIn: "100",
        stableSwapOut: "100",
        addLiquidity: "100",
        removeLiquidity: "100",
        openPosition: "100",
        closePosition: "100",
        liquidation: "100",
        protocolShare: "10",
      },
      borrowRate: {
        baseRate: "0",
        slope1: "80000",
        slope2: "120000",
        optimalUtilization: "800000000",
      },
      assets: {
        collateral: "0",
        protocolFees: "0",
        owned: "0",
        locked: "0",
      },
      collectedFees: {
        swapUsd: "0",
        addLiquidityUsd: "0",
        removeLiquidityUsd: "0",
        openPositionUsd: "0",
        closePositionUsd: "0",
        liquidationUsd: "0",
      },
      volumeStats: {
        swapUsd: "0",
        addLiquidityUsd: "0",
        removeLiquidityUsd: "0",
        openPositionUsd: "0",
        closePositionUsd: "0",
        liquidationUsd: "0",
      },
      tradeStats: {
        profitUsd: "0",
        lossUsd: "0",
        oiLongUsd: "0",
        oiShortUsd: "0",
      },
      longPositions: {
        openPositions: "0",
        collateralUsd: "0",
        sizeUsd: "0",
        lockedAmount: "0",
        weightedPrice: "0",
        totalQuantity: "0",
        cumulativeInterestUsd: "0",
        cumulativeInterestSnapshot: "0",
      },
      shortPositions: {
        openPositions: "0",
        collateralUsd: "0",
        sizeUsd: "0",
        lockedAmount: "0",
        weightedPrice: "0",
        totalQuantity: "0",
        cumulativeInterestUsd: "0",
        cumulativeInterestSnapshot: "0",
      },
      borrowRateState: {
        currentRate: "0",
        cumulativeInterest: "0",
        lastUpdate: "0",
      },
      bump: token.bump,
      tokenAccountBump: token.tokenAccountBump,
    };
    expect(JSON.stringify(token)).to.equal(JSON.stringify(tokenExpected));

    let oracleConfig2 = Object.assign({}, oracleConfig);
    oracleConfig2.oracleAccount = tc.custodies[1].oracleAccount;
    await tc.addCustody(
      tc.custodies[1],
      isStable,
      oracleConfig2,
      pricing,
      permissions,
      fees,
      borrowRate,
      ratios
    );

    await tc.removeCustody(tc.custodies[1], ratios1);
    tc.ensureFails(tc.program.account.custody.fetch(tc.custodies[1].custody));

    await tc.addCustody(
      tc.custodies[1],
      isStable,
      oracleConfig2,
      pricing,
      permissions,
      fees,
      borrowRate,
      ratios
    );
  });

  it("setCustodyConfig", async () => {
    oracleConfig.maxPriceAgeSec = 90;
    permissions.allowPnlWithdrawal = false;
    fees.liquidation = new BN(200);
    ratios[0].min = new BN(90);
    await tc.setCustodyConfig(
      tc.custodies[0],
      isStable,
      oracleConfig,
      pricing,
      permissions,
      fees,
      borrowRate,
      ratios
    );

    let token = await tc.program.account.custody.fetch(tc.custodies[0].custody);
    tokenExpected.oracle.maxPriceAgeSec = 90;
    tokenExpected.permissions.allowPnlWithdrawal = false;
    tokenExpected.fees.liquidation = "200";
    expect(JSON.stringify(token)).to.equal(JSON.stringify(tokenExpected));
  });

  it("setTestOraclePrice", async () => {
    await tc.setTestOraclePrice(123, tc.custodies[0]);
    await tc.setTestOraclePrice(200, tc.custodies[1]);

    let oracle = await tc.program.account.testOracle.fetch(
      tc.custodies[0].oracleAccount
    );
    let oracleExpected = {
      price: new BN(123000),
      expo: -3,
      conf: new BN(0),
      publishTime: oracle.publishTime,
    };
    expect(JSON.stringify(oracle)).to.equal(JSON.stringify(oracleExpected));
  });

  it("setTestTime", async () => {
    await tc.setTestTime(111);

    let perpetuals = await tc.program.account.perpetuals.fetch(
      tc.perpetuals.publicKey
    );
    expect(JSON.stringify(perpetuals.inceptionTime)).to.equal(
      JSON.stringify(new BN(111))
    );
  });

  it("addLiquidity", async () => {
    await tc.addLiquidity(
      tc.toTokenAmount(10, tc.custodies[0].decimals),
      new BN(1),
      tc.users[0],
      tc.users[0].tokenAccounts[0],
      tc.custodies[0]
    );
    await tc.addLiquidity(
      tc.toTokenAmount(10, tc.custodies[1].decimals),
      new BN(1),
      tc.users[1],
      tc.users[1].tokenAccounts[1],
      tc.custodies[1]
    );
  });

  it("swap", async () => {
    await tc.swap(
      tc.toTokenAmount(1, tc.custodies[0].decimals),
      new BN(1),
      tc.users[0],
      tc.users[0].tokenAccounts[0],
      tc.users[0].tokenAccounts[1],
      tc.custodies[0],
      tc.custodies[1]
    );
  });

  it("removeLiquidity", async () => {
    await tc.removeLiquidity(
      tc.toTokenAmount(1, 6),
      new BN(1),
      tc.users[0],
      tc.users[0].tokenAccounts[0],
      tc.custodies[0]
    );
    await tc.removeLiquidity(
      tc.toTokenAmount(1, 6),
      new BN(1),
      tc.users[1],
      tc.users[1].tokenAccounts[1],
      tc.custodies[1]
    );
  });

  it("openPosition", async () => {
    await tc.openPosition(
      125,
      tc.toTokenAmount(1, tc.custodies[0].decimals),
      tc.toTokenAmount(7, tc.custodies[0].decimals),
      "long",
      tc.users[0],
      tc.users[0].tokenAccounts[0],
      tc.users[0].positionAccountsLong[0],
      tc.custodies[0]
    );

    let position = await tc.program.account.position.fetch(
      tc.users[0].positionAccountsLong[0]
    );
    positionExpected = {
      owner: tc.users[0].wallet.publicKey.toBase58(),
      pool: tc.pool.publicKey.toBase58(),
      custody: tc.custodies[0].custody.toBase58(),
      openTime: "111",
      updateTime: "0",
      side: { long: {} },
      price: "124230000",
      sizeUsd: "861000000",
      collateralUsd: "123000000",
      unrealizedProfitUsd: "0",
      unrealizedLossUsd: "0",
      cumulativeInterestSnapshot: "0",
      lockedAmount: "7000000000",
      collateralAmount: "1000000000",
      bump: position.bump,
    };

    expect(JSON.stringify(position)).to.equal(JSON.stringify(positionExpected));
  });

  it("addCollateral", async () => {
    await tc.addCollateral(
      tc.toTokenAmount(1, tc.custodies[0].decimals),
      tc.users[0],
      tc.users[0].tokenAccounts[0],
      tc.users[0].positionAccountsLong[0],
      tc.custodies[0]
    );
  });

  it("removeCollateral", async () => {
    await tc.removeCollateral(
      tc.toTokenAmount(1, 6),
      tc.users[0],
      tc.users[0].tokenAccounts[0],
      tc.users[0].positionAccountsLong[0],
      tc.custodies[0]
    );
  });

  it("closePosition", async () => {
    await tc.closePosition(
      1,
      tc.users[0],
      tc.users[0].tokenAccounts[0],
      tc.users[0].positionAccountsLong[0],
      tc.custodies[0]
    );
    tc.ensureFails(
      tc.program.account.position.fetch(tc.users[0].positionAccountsLong[0])
    );
  });

  it("liquidate", async () => {
    await tc.openPosition(
      125,
      tc.toTokenAmount(1, tc.custodies[0].decimals),
      tc.toTokenAmount(7, tc.custodies[0].decimals),
      "long",
      tc.users[0],
      tc.users[0].tokenAccounts[0],
      tc.users[0].positionAccountsLong[0],
      tc.custodies[0]
    );
    await tc.setTestOraclePrice(80, tc.custodies[0]);
    await tc.liquidate(
      tc.users[0],
      tc.users[0].tokenAccounts[0],
      tc.users[0].positionAccountsLong[0],
      tc.custodies[0]
    );
    tc.ensureFails(
      tc.program.account.position.fetch(tc.users[0].positionAccountsLong[0])
    );
  });
});
