import * as anchor from "@project-serum/anchor";
import { Perpetuals } from "../target/types/perpetuals";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  AccountMeta,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_SLOT_HASHES_PUBKEY,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  AddressLookupTableProgram,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import * as spl from "@solana/spl-token";
import { BN } from "bn.js";

export type PositionSide = "long" | "short";

export class TestClient {
  provider: anchor.AnchorProvider;
  program: anchor.Program<Perpetuals>;
  printErrors: boolean;

  admins: Keypair[];
  feesAccount: PublicKey;
  adminMetas: AccountMeta[];

  // pdas
  multisig: { publicKey: PublicKey; bump: number };
  authority: { publicKey: PublicKey; bump: number };
  perpetuals: { publicKey: PublicKey; bump: number };
  pool: { publicKey: PublicKey; bump: number };
  lpToken: { publicKey: PublicKey; bump: number };

  custodies: {
    mint: Keypair;
    tokenAccount: PublicKey;
    oracleAccount: PublicKey;
    custody: PublicKey;
    decimals: number;
  }[];
  custodyMetas: AccountMeta[];

  users: {
    wallet: Keypair;
    tokenAccounts: PublicKey[];
    lpTokenAccount: PublicKey;
    positionAccountsLong: PublicKey[];
    positionAccountsShort: PublicKey[];
  }[];

  constructor() {
    this.provider = anchor.AnchorProvider.env();
    anchor.setProvider(this.provider);
    this.program = anchor.workspace.Perpetuals as Program<Perpetuals>;
    this.printErrors = true;

    anchor.BN.prototype.toJSON = function () {
      return this.toString(10);
    };
  }

  initFixture = async () => {
    // fixed addresses
    this.admins = [];
    this.admins.push(Keypair.generate());
    this.admins.push(Keypair.generate());

    this.adminMetas = [];
    for (const admin of this.admins) {
      this.adminMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: admin.publicKey,
      });
    }

    // pdas
    this.multisig = this.findProgramAddress("multisig");
    this.authority = this.findProgramAddress("transfer_authority");
    this.perpetuals = this.findProgramAddress("perpetuals");
    this.pool = this.findProgramAddress("pool", "test pool");
    this.lpToken = this.findProgramAddress("lp_token_mint", [
      this.pool.publicKey,
    ]);

    // custodies
    this.custodies = [];
    this.custodies.push(this.generateCustody(9));
    this.custodies.push(this.generateCustody(6));

    this.custodyMetas = [];
    for (const custody of this.custodies) {
      this.custodyMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: custody.custody,
      });
    }
    for (const custody of this.custodies) {
      this.custodyMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: custody.oracleAccount,
      });
    }

    // airdrop funds
    await this.confirmTx(await this.requestAirdrop(this.admins[0].publicKey));

    // create mints
    for (const custody of this.custodies) {
      await spl.createMint(
        this.provider.connection,
        this.admins[0],
        this.admins[0].publicKey,
        null,
        custody.decimals,
        custody.mint
      );
    }

    // fees receiving account
    this.feesAccount = await spl.createAssociatedTokenAccount(
      this.provider.connection,
      this.admins[0],
      this.custodies[0].mint.publicKey,
      this.admins[0].publicKey
    );

    // users
    this.users = [];
    for (let i = 0; i < 2; ++i) {
      let wallet = Keypair.generate();
      await this.requestAirdrop(wallet.publicKey);

      let tokenAccounts = [];
      let positionAccountsLong = [];
      let positionAccountsShort = [];
      for (const custody of this.custodies) {
        let tokenAccount = await spl.createAssociatedTokenAccount(
          this.provider.connection,
          this.admins[0],
          custody.mint.publicKey,
          wallet.publicKey
        );
        await this.mintTokens(
          1000,
          custody.decimals,
          custody.mint.publicKey,
          tokenAccount
        );
        tokenAccounts.push(tokenAccount);

        let positionAccount = this.findProgramAddress("position", [
          wallet.publicKey,
          this.pool.publicKey,
          custody.custody,
          [1],
        ]).publicKey;
        positionAccountsLong.push(positionAccount);

        positionAccount = this.findProgramAddress("position", [
          wallet.publicKey,
          this.pool.publicKey,
          custody.custody,
          [2],
        ]).publicKey;
        positionAccountsShort.push(positionAccount);
      }

      this.users.push({
        wallet,
        tokenAccounts,
        lpTokenAccount: PublicKey.default,
        positionAccountsLong,
        positionAccountsShort,
      });
    }
  };

  requestAirdrop = async (pubkey: PublicKey) => {
    if ((await this.getSolBalance(pubkey)) < 1e9 / 2) {
      return this.provider.connection.requestAirdrop(pubkey, 1e9);
    }
  };

  mintTokens = async (
    uiAmount: number,
    decimals: number,
    mint: PublicKey,
    destiantionWallet: PublicKey
  ) => {
    await spl.mintToChecked(
      this.provider.connection,
      this.admins[0],
      mint,
      destiantionWallet,
      this.admins[0],
      this.toTokenAmount(uiAmount, decimals).toNumber(),
      decimals
    );
  };

  generateCustody = (decimals: number) => {
    let mint = Keypair.generate();
    let tokenAccount = this.findProgramAddress("custody_token_account", [
      this.pool.publicKey,
      mint.publicKey,
    ]).publicKey;
    let oracleAccount = this.findProgramAddress("oracle_account", [
      this.pool.publicKey,
      mint.publicKey,
    ]).publicKey;
    let custody = this.findProgramAddress("custody", [
      this.pool.publicKey,
      mint.publicKey,
    ]).publicKey;
    return {
      mint,
      tokenAccount,
      oracleAccount,
      custody,
      decimals,
    };
  };

  findProgramAddress = (label: string, extraSeeds = null) => {
    let seeds = [Buffer.from(anchor.utils.bytes.utf8.encode(label))];
    if (extraSeeds) {
      for (let extraSeed of extraSeeds) {
        if (typeof extraSeed === "string") {
          seeds.push(Buffer.from(anchor.utils.bytes.utf8.encode(extraSeed)));
        } else if (Array.isArray(extraSeed)) {
          seeds.push(Buffer.from(extraSeed));
        } else {
          seeds.push(extraSeed.toBuffer());
        }
      }
    }
    let res = PublicKey.findProgramAddressSync(seeds, this.program.programId);
    return { publicKey: res[0], bump: res[1] };
  };

  confirmTx = async (txSignature: anchor.web3.TransactionSignature) => {
    const latestBlockHash = await this.provider.connection.getLatestBlockhash();

    await this.provider.connection.confirmTransaction(
      {
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: txSignature,
      },
      { commitment: "processed" }
    );
  };

  confirmAndLogTx = async (txSignature: anchor.web3.TransactionSignature) => {
    await this.confirmTx(txSignature);
    let tx = await this.provider.connection.getTransaction(txSignature, {
      commitment: "confirmed",
    });
    console.log(tx);
  };

  getBalance = async (pubkey: PublicKey) => {
    return spl
      .getAccount(this.provider.connection, pubkey)
      .then((account) => Number(account.amount))
      .catch(() => 0);
  };

  getSolBalance = async (pubkey: PublicKey) => {
    return this.provider.connection
      .getBalance(pubkey)
      .then((balance) => balance)
      .catch(() => 0);
  };

  getExtraSolBalance = async (pubkey: PublicKey) => {
    let balance = await this.provider.connection
      .getBalance(pubkey)
      .then((balance) => balance)
      .catch(() => 0);
    let accountInfo = await this.provider.connection.getAccountInfo(pubkey);
    let dataSize = accountInfo ? accountInfo.data.length : 0;
    let minBalance =
      await this.provider.connection.getMinimumBalanceForRentExemption(
        dataSize
      );
    return balance > minBalance ? balance - minBalance : 0;
  };

  getTokenAccount = async (pubkey: PublicKey) => {
    return spl.getAccount(this.provider.connection, pubkey);
  };

  getTime() {
    const now = new Date();
    const utcMilllisecondsSinceEpoch =
      now.getTime() + now.getTimezoneOffset() * 60 * 1000;
    return utcMilllisecondsSinceEpoch / 1000;
  }

  toTokenAmount(uiAmount: number, decimals: number) {
    return new BN(uiAmount * 10 ** decimals);
  }

  toUiAmount(token_amount: number, decimals: number) {
    return token_amount / 10 ** decimals;
  }

  ensureFails = async (promise, message = null) => {
    let printErrors = this.printErrors;
    this.printErrors = false;
    let res = null;
    try {
      await promise;
    } catch (err) {
      res = err;
    }
    this.printErrors = printErrors;
    if (!res) {
      throw new Error(message ? message : "Call should've failed");
    }
    return res;
  };

  ///////
  // instructions

  init = async () => {
    try {
      await this.program.methods
        .testInit({
          minSignatures: 2,
          allowSwap: true,
          allowAddLiquidity: true,
          allowRemoveLiquidity: true,
          allowOpenPosition: true,
          allowClosePosition: true,
          allowPnlWithdrawal: true,
          allowCollateralWithdrawal: true,
          allowSizeChange: true,
        })
        .accounts({
          upgradeAuthority: this.provider.wallet.publicKey,
          multisig: this.multisig.publicKey,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .remainingAccounts(this.adminMetas)
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  setAdminSigners = async (minSignatures: number) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setAdminSigners({
            minSignatures,
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
          })
          .remainingAccounts(this.adminMetas)
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  setPermissions = async (permissions) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setPermissions(permissions)
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            perpetuals: this.perpetuals.publicKey,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  addPool = async (name) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .addPool({ name })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            transferAuthority: this.authority.publicKey,
            perpetuals: this.perpetuals.publicKey,
            pool: this.pool.publicKey,
            lpTokenMint: this.lpToken.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: spl.TOKEN_PROGRAM_ID,
            rent: SYSVAR_RENT_PUBKEY,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }

    // set lp token accounts
    for (let i = 0; i < 2; ++i) {
      let tokenAccount = await spl.getOrCreateAssociatedTokenAccount(
        this.provider.connection,
        this.admins[0],
        this.lpToken.publicKey,
        this.users[i].wallet.publicKey
      );
      this.users[i].lpTokenAccount = tokenAccount.address;
    }
  };

  removePool = async () => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .removePool({})
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            transferAuthority: this.authority.publicKey,
            perpetuals: this.perpetuals.publicKey,
            pool: this.pool.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  addCustody = async (
    custody,
    isStable,
    oracleConfig,
    pricing,
    permissions,
    fees,
    borrowRate,
    ratios
  ) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .addCustody({
            isStable,
            oracle: oracleConfig,
            pricing,
            permissions,
            fees,
            borrowRate,
            ratios,
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            transferAuthority: this.authority.publicKey,
            perpetuals: this.perpetuals.publicKey,
            pool: this.pool.publicKey,
            custody: custody.custody,
            custodyTokenAccount: custody.tokenAccount,
            custodyTokenMint: custody.mint.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: spl.TOKEN_PROGRAM_ID,
            rent: SYSVAR_RENT_PUBKEY,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  removeCustody = async (custody, ratios) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .removeCustody({ ratios })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            transferAuthority: this.authority.publicKey,
            perpetuals: this.perpetuals.publicKey,
            pool: this.pool.publicKey,
            custody: custody.custody,
            custodyTokenAccount: custody.tokenAccount,
            systemProgram: SystemProgram.programId,
            tokenProgram: spl.TOKEN_PROGRAM_ID,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  setCustodyConfig = async (
    custody,
    isStable,
    oracleConfig,
    pricing,
    permissions,
    fees,
    borrowRate,
    ratios
  ) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setCustodyConfig({
            isStable,
            oracle: oracleConfig,
            pricing,
            permissions,
            fees,
            borrowRate,
            ratios,
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            pool: this.pool.publicKey,
            custody: custody.custody,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  withdrawFees = async (amount: typeof BN, custody, receivingTokenAccount) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .withdrawFees({
            amount,
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            transferAuthority: this.authority.publicKey,
            perpetuals: this.perpetuals.publicKey,
            pool: this.pool.publicKey,
            custody: custody.custody,
            custodyTokenAccount: custody.tokenAccount,
            receivingTokenAccount: receivingTokenAccount,
            tokenProgram: spl.TOKEN_PROGRAM_ID,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  withdrawSolFees = async (amount: typeof BN, custody, receivingAccount) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .withdrawSolFees({
            amount,
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            transferAuthority: this.authority.publicKey,
            perpetuals: this.perpetuals.publicKey,
            receivingAccount: receivingAccount,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  setTestOraclePrice = async (price: number, custody) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setTestOraclePrice({
            price: new BN(price * 1000),
            expo: -3,
            conf: new BN(0),
            publishTime: new BN(this.getTime()),
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            perpetuals: this.perpetuals.publicKey,
            pool: this.pool.publicKey,
            custody: custody.custody,
            oracleAccount: custody.oracleAccount,
            systemProgram: SystemProgram.programId,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  setTestTime = async (time: number) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setTestTime({
            time: new BN(time),
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            perpetuals: this.perpetuals.publicKey,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  swap = async (
    amountIn: typeof BN,
    minAmountOut: typeof BN,
    user,
    fundingAccount: PublicKey,
    receivingAccount: PublicKey,
    custodyIn,
    custodyOut
  ) => {
    try {
      await this.program.methods
        .swap({
          amountIn,
          minAmountOut,
        })
        .accounts({
          owner: user.wallet.publicKey,
          fundingAccount: fundingAccount,
          receivingAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          receivingCustody: custodyIn.custody,
          receivingCustodyOracleAccount: custodyIn.oracleAccount,
          receivingCustodyTokenAccount: custodyIn.tokenAccount,
          dispensingCustody: custodyOut.custody,
          dispensingCustodyOracleAccount: custodyOut.oracleAccount,
          dispensingCustodyTokenAccount: custodyOut.tokenAccount,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  addLiquidity = async (
    amountIn: typeof BN,
    minLpAmountOut: typeof BN,
    user,
    fundingAccount: PublicKey,
    custody
  ) => {
    try {
      await this.program.methods
        .addLiquidity({
          amountIn,
          minLpAmountOut,
        })
        .accounts({
          owner: user.wallet.publicKey,
          fundingAccount,
          lpTokenAccount: user.lpTokenAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          lpTokenMint: this.lpToken.publicKey,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .remainingAccounts(this.custodyMetas)
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  removeLiquidity = async (
    lpAmountIn: typeof BN,
    minAmountOut: typeof BN,
    user,
    receivingAccount: PublicKey,
    custody
  ) => {
    try {
      await this.program.methods
        .removeLiquidity({
          lpAmountIn,
          minAmountOut,
        })
        .accounts({
          owner: user.wallet.publicKey,
          receivingAccount: receivingAccount,
          lpTokenAccount: user.lpTokenAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          lpTokenMint: this.lpToken.publicKey,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .remainingAccounts(this.custodyMetas)
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  openPosition = async (
    price: number,
    collateral: typeof BN,
    size: typeof BN,
    side: PositionSide,
    user,
    fundingAccount: PublicKey,
    positionAccount: PublicKey,
    custody
  ) => {
    try {
      await this.program.methods
        .openPosition({
          price: new BN(price * 1000000),
          collateral,
          size,
          side: side === "long" ? { long: {} } : { short: {} },
        })
        .accounts({
          owner: user.wallet.publicKey,
          fundingAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          position: positionAccount,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  addCollateral = async (
    collateral: typeof BN,
    user,
    fundingAccount: PublicKey,
    positionAccount: PublicKey,
    custody
  ) => {
    try {
      await this.program.methods
        .addCollateral({
          collateral,
        })
        .accounts({
          owner: user.wallet.publicKey,
          fundingAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          position: positionAccount,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  removeCollateral = async (
    collateralUsd: typeof BN,
    user,
    receivingAccount: PublicKey,
    positionAccount: PublicKey,
    custody
  ) => {
    try {
      await this.program.methods
        .removeCollateral({
          collateralUsd,
        })
        .accounts({
          owner: user.wallet.publicKey,
          receivingAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          position: positionAccount,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  closePosition = async (
    price: number,
    user,
    receivingAccount,
    positionAccount,
    custody
  ) => {
    try {
      await this.program.methods
        .closePosition({
          price: new BN(price),
        })
        .accounts({
          owner: user.wallet.publicKey,
          receivingAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          position: positionAccount,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  liquidate = async (
    user,
    tokenAccount: PublicKey,
    positionAccount: PublicKey,
    custody
  ) => {
    try {
      await this.program.methods
        .liquidate({})
        .accounts({
          signer: user.wallet.publicKey,
          receivingAccount: tokenAccount,
          rewardsReceivingAccount: tokenAccount,
          transferAuthority: this.authority.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          position: positionAccount,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
          custodyTokenAccount: custody.tokenAccount,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  getEntryPriceAndFee = async (
    size: typeof BN,
    side: PositionSide,
    custody
  ) => {
    try {
      return await this.program.methods
        .getEntryPriceAndFee({
          size,
          side: side === "long" ? { long: {} } : { short: {} },
        })
        .accounts({
          signer: this.provider.wallet.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
        })
        .view();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  getExitPriceAndFee = async (
    size: typeof BN,
    positionAccount: PublicKey,
    custody
  ) => {
    try {
      return await this.program.methods
        .getExitPriceAndFee({
          size,
        })
        .accounts({
          signer: this.provider.wallet.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          position: positionAccount,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
        })
        .view();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  getLiquidationPrice = async (positionAccount: PublicKey, custody) => {
    try {
      return await this.program.methods
        .getLiquidationPrice({})
        .accounts({
          signer: this.provider.wallet.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          position: positionAccount,
          custody: custody.custody,
          custodyOracleAccount: custody.oracleAccount,
        })
        .view();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  getSwapAmountAndFee = async (amountIn: number, custodyIn, custodyOut) => {
    try {
      return await this.program.methods
        .getSwapAmountAndFee({
          amountIn: new BN(amountIn),
        })
        .accounts({
          signer: this.provider.wallet.publicKey,
          perpetuals: this.perpetuals.publicKey,
          pool: this.pool.publicKey,
          receivingCustody: custodyIn.custody,
          receivingCustodyOracleAccount: custodyIn.oracleAccount,
          receivingCustodyTokenAccount: custodyIn.tokenAccount,
          dispensingCustody: custodyOut.custody,
          dispensingCustodyOracleAccount: custodyOut.oracleAccount,
          dispensingCustodyTokenAccount: custodyOut.tokenAccount,
        })
        .view();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };
}
