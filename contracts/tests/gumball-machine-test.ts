import * as anchor from "@project-serum/anchor";
import { BN, AnchorProvider, Program } from "@project-serum/anchor";
import { Bubblegum } from "../target/types/bubblegum";
import { GumballMachine } from "../target/types/gumball_machine";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  LAMPORTS_PER_SOL,
  ComputeBudgetProgram,
  Connection,
} from "@solana/web3.js";
import { assert } from "chai";

import { buildTree } from "./merkle-tree";
import {
  getMerkleRollAccountSize,
  assertOnChainMerkleRollProperties,
} from "@sorend-solana/gummyroll";
import {
  decodeGumballMachine,
  OnChainGumballMachine,
  createDispenseNFTForSolIx,
  createDispenseNFTForTokensIx,
  createInitializeGumballMachineIxs,
  initializeGumballMachineIndices,
  InitializeGumballMachineInstructionArgs,
  createAddConfigLinesInstruction,
  createUpdateConfigLinesInstruction,
  UpdateHeaderMetadataInstructionArgs,
  UpdateConfigLinesInstructionArgs,
  createUpdateHeaderMetadataInstruction,
  createDestroyInstruction,
  GumballMachineHeader,
  gumballMachineHeaderBeet,
  GumballCreatorAdapter,
  EncodeMethod
} from "@sorend-solana/gumball-machine";
import {
  val,
  strToByteArray,
  strToByteUint8Array,
  num32ToBuffer,
  arrayEquals,
  logTx,
  execute
} from "@sorend-solana/utils";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
} from "../../deps/solana-program-library/token/js/src";
import { NATIVE_MINT } from "@solana/spl-token";
import { getBubblegumAuthorityPDA } from "@sorend-solana/bubblegum";

// @ts-ignore
let GumballMachine;
let Bubblegum;
// @ts-ignore
let GummyrollProgramId;
let BubblegumProgramId;

describe("gumball-machine", function () {
  // Configure the client to use the local cluster.
  let payer: Keypair;
  let connection: Connection;
  let wallet: NodeWallet;
  let baseGumballMachineInitProps: InitializeGumballMachineInstructionArgs;
  let creatorAddress: Keypair;
  let gumballMachineAcctKeypair: Keypair;
  let merkleRollKeypair: Keypair;
  let nftBuyer: Keypair;
  let creatorKeys: PublicKey[];
  let creatorShares: Uint8Array;

  function assertGumballMachineHeaderProperties(
    gm: OnChainGumballMachine,
    expectedHeader: GumballMachineHeader
  ) {
    assert(
      arrayEquals(gm.header.urlBase, expectedHeader.urlBase),
      "Gumball Machine has incorrect url base"
    );
    assert(
      arrayEquals(gm.header.nameBase, expectedHeader.nameBase),
      "Gumball Machine has incorrect name base"
    );
    assert(
      arrayEquals(gm.header.symbol, expectedHeader.symbol),
      "Gumball Machine has incorrect symbol"
    );
    assert(
      gm.header.sellerFeeBasisPoints === expectedHeader.sellerFeeBasisPoints,
      "Gumball Machine has seller fee basis points"
    );
    assert(
      gm.header.isMutable === expectedHeader.isMutable,
      "Gumball Machine has incorrect isMutable"
    );
    assert(
      gm.header.retainAuthority === expectedHeader.retainAuthority,
      "Gumball Machine has incorrect retainAuthority"
    );
    assert(
      val(gm.header.price).eq(val(expectedHeader.price)),
      "Gumball Machine has incorrect price"
    );
    assert(
      val(gm.header.goLiveDate).eq(val(expectedHeader.goLiveDate)),
      "Gumball Machine has incorrect goLiveDate"
    );
    assert(
      gm.header.mint.equals(expectedHeader.mint),
      "Gumball Machine set with incorrect mint"
    );
    assert(
      gm.header.botWallet.equals(expectedHeader.botWallet),
      "Gumball Machine set with incorrect botWallet"
    );
    assert(
      gm.header.receiver.equals(expectedHeader.receiver),
      "Gumball Machine set with incorrect receiver"
    );
    assert(
      gm.header.authority.equals(expectedHeader.authority),
      "Gumball Machine set with incorrect authority"
    );
    assert(
      gm.header.collectionKey.equals(expectedHeader.collectionKey),
      "Gumball Machine set with incorrect collectionKey"
    );
    assert(
      gm.header.mint.equals(expectedHeader.mint),
      "Gumball Machine set with incorrect mint"
    );
    assert(
      val(gm.header.extensionLen).eq(val(expectedHeader.extensionLen)),
      "Gumball Machine has incorrect extensionLen"
    );
    assert(
      gm.header.maxMintSize === expectedHeader.maxMintSize,
      "Gumball Machine has incorrect maxMintSize"
    );
    assert(
      gm.header.maxItems === expectedHeader.maxItems,
      "Gumball Machine has incorrect max items"
    );
    for (let i = 0; i < gm.header.creators.length; i++) {
      // Check that creator matches user specification
      if (i < expectedHeader.creators.length) {
        assert(
          gm.header.creators[i].address.equals(
            expectedHeader.creators[i].address
          ),
          "Gumball Machine creator has mismatching address"
        );
        assert(
          gm.header.creators[i].share === expectedHeader.creators[i].share,
          "Gumball Machine creator has mismatching share"
        );
        assert(
          gm.header.creators[i].verified ===
          expectedHeader.creators[i].verified,
          "Gumball Machine creator has mismatching verified field"
        );
      }
      // Check that non-user specified creators are default
      else {
        assert(
          gm.header.creators[i].address.equals(
            new PublicKey("11111111111111111111111111111111")
          ),
          "Gumball Machine creator has mismatching address"
        );
        assert(
          gm.header.creators[i].share === 0,
          "Gumball Machine creator has mismatching share"
        );
        assert(
          gm.header.creators[i].verified === 0,
          "Gumball Machine creator has mismatching verified field"
        );
      }
    }
  }

  function assertGumballMachineConfigProperties(
    gm: OnChainGumballMachine,
    expectedIndexArray: Buffer,
    expectedConfigLines: Buffer = null,
    onChainConfigLinesNumBytes: number = null
  ) {
    assert(
      gm.configData.indexArray.equals(expectedIndexArray),
      "Onchain index array doesn't match expectation"
    );

    if (expectedConfigLines && onChainConfigLinesNumBytes) {
      // Calculate full-sized on-chain config bytes buffer, we must null pad the buffer up to the end of the account size
      const numExpectedInitializedBytesInConfig =
        expectedConfigLines.byteLength;
      const bufferOfNonInitializedConfigLineBytes = Buffer.from(
        "\0".repeat(
          onChainConfigLinesNumBytes - numExpectedInitializedBytesInConfig
        )
      );
      const actualExpectedConfigLinesBuffer = Buffer.concat([
        expectedConfigLines,
        bufferOfNonInitializedConfigLineBytes,
      ]);
      assert(
        gm.configData.configLines.equals(actualExpectedConfigLinesBuffer),
        "Config lines on gumball machine do not match expectation"
      );
    }
  }

  async function initializeGumballMachine(
    payer: Keypair,
    gumballMachineAcctKeypair: Keypair,
    gumballMachineAcctSize: number,
    merkleRollKeypair: Keypair,
    merkleRollAccountSize: number,
    gumballMachineInitArgs: InitializeGumballMachineInstructionArgs,
    mint: PublicKey
  ) {
    const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDA(
      merkleRollKeypair.publicKey
    );
    const initializeGumballMachineInstrs =
      await createInitializeGumballMachineIxs(
        payer.publicKey,
        gumballMachineAcctKeypair.publicKey,
        gumballMachineAcctSize,
        merkleRollKeypair.publicKey,
        merkleRollAccountSize,
        gumballMachineInitArgs,
        mint,
        GumballMachine.provider.connection
      );
    const tx = new Transaction();
    initializeGumballMachineInstrs.forEach((instr) => tx.add(instr));
    await execute(GumballMachine.provider, initializeGumballMachineInstrs, [payer, gumballMachineAcctKeypair, merkleRollKeypair]);

    const tree = buildTree(
      Array(2 ** gumballMachineInitArgs.maxDepth).fill(Buffer.alloc(32))
    );
    await assertOnChainMerkleRollProperties(
      GumballMachine.provider.connection,
      gumballMachineInitArgs.maxDepth,
      gumballMachineInitArgs.maxBufferSize,
      bubblegumAuthorityPDAKey,
      new PublicKey(tree.root),
      merkleRollKeypair.publicKey
    );

    const onChainGumballMachineAccount =
      await GumballMachine.provider.connection.getAccountInfo(
        gumballMachineAcctKeypair.publicKey
      );

    const gumballMachine = decodeGumballMachine(
      onChainGumballMachineAccount.data,
      gumballMachineAcctSize
    );

    let expectedCreators = [];
    for (let i = 0; i < gumballMachineInitArgs.creatorKeys.length; i++) {
      let c: GumballCreatorAdapter = {
        address: gumballMachineInitArgs.creatorKeys[i],
        share: gumballMachineInitArgs.creatorShares[i],
        verified: 0,
      };
      expectedCreators.push(c);
    }
    gumballMachineInitArgs.creatorKeys;
    let expectedOnChainHeader: GumballMachineHeader = {
      urlBase: gumballMachineInitArgs.urlBase,
      nameBase: gumballMachineInitArgs.nameBase,
      symbol: gumballMachineInitArgs.symbol,
      sellerFeeBasisPoints: gumballMachineInitArgs.sellerFeeBasisPoints,
      isMutable: gumballMachineInitArgs.isMutable ? 1 : 0,
      retainAuthority: gumballMachineInitArgs.retainAuthority ? 1 : 0,
      configLineEncodeMethod: 0,
      creators: expectedCreators,
      price: gumballMachineInitArgs.price,
      goLiveDate: gumballMachineInitArgs.goLiveDate,
      mint,
      botWallet: gumballMachineInitArgs.botWallet,
      receiver: gumballMachineInitArgs.receiver,
      authority: gumballMachineInitArgs.authority,
      collectionKey: gumballMachineInitArgs.collectionKey,
      extensionLen: gumballMachineInitArgs.extensionLen,
      maxMintSize: gumballMachineInitArgs.maxMintSize,
      remaining: 0,
      maxItems: gumballMachineInitArgs.maxItems,
      totalItemsAdded: 0,
      smallestUninitializedIndex: 0,
      padding: [0, 0, 0, 0, 0, 0, 0]
    };
    assertGumballMachineHeaderProperties(gumballMachine, expectedOnChainHeader);
  }

  async function initializeIndicesAndAssert(
    maxItems: number,
    authority: Keypair,
    gumballMachine: PublicKey,
    gumballMachineAcctSize: number
  ) {
    // Initialize all indices
    await initializeGumballMachineIndices(
      GumballMachine.provider,
      maxItems,
      authority,
      gumballMachine
    );
    const onChainGumballMachineAccount =
      await GumballMachine.provider.connection.getAccountInfo(gumballMachine);
    const onChainGumballMachine = decodeGumballMachine(
      onChainGumballMachineAccount.data,
      gumballMachineAcctSize
    );

    // Create the expected buffer for the indices of the account
    const expectedIndexArrBuffer = [...Array(maxItems).keys()].reduce(
      (prevVal, curVal) =>
        Buffer.concat([prevVal, Buffer.from(num32ToBuffer(curVal))]),
      Buffer.from([])
    );

    assertGumballMachineConfigProperties(
      onChainGumballMachine,
      expectedIndexArrBuffer
    );
  }

  async function addConfigLines(
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    gumballMachineAcctSize: number,
    gumballMachineAcctConfigIndexArrSize: number,
    gumballMachineAcctConfigLinesSize: number,
    configLinesToAdd: Uint8Array,
    allExpectedInitializedConfigLines: Buffer
  ) {
    const addConfigLinesInstr = createAddConfigLinesInstruction(
      {
        gumballMachine: gumballMachineAcctKey,
        authority: authority.publicKey,
      },
      {
        newConfigLinesData: configLinesToAdd,
      }
    );
    await execute(GumballMachine.provider, [addConfigLinesInstr], [authority]);
    const onChainGumballMachineAccount =
      await GumballMachine.provider.connection.getAccountInfo(
        gumballMachineAcctKey
      );
    const gumballMachine = decodeGumballMachine(
      onChainGumballMachineAccount.data,
      gumballMachineAcctSize
    );

    // Create the expected buffer for the indices of the account
    const expectedIndexArrBuffer = [
      ...Array(gumballMachineAcctConfigIndexArrSize / 4).keys(),
    ].reduce(
      (prevVal, curVal) =>
        Buffer.concat([prevVal, Buffer.from(num32ToBuffer(curVal))]),
      Buffer.from([])
    );

    assertGumballMachineConfigProperties(
      gumballMachine,
      expectedIndexArrBuffer,
      allExpectedInitializedConfigLines,
      gumballMachineAcctConfigLinesSize
    );
  }

  async function updateConfigLines(
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    gumballMachineAcctSize,
    gumballMachineAcctConfigIndexArrSize: number,
    gumballMachineAcctConfigLinesSize: number,
    updatedConfigLines: Buffer,
    allExpectedInitializedConfigLines: Buffer,
    indexOfFirstLineToUpdate: BN
  ) {
    const args: UpdateConfigLinesInstructionArgs = {
      startingLine: indexOfFirstLineToUpdate,
      newConfigLinesData: updatedConfigLines,
    };
    const updateConfigLinesInstr = createUpdateConfigLinesInstruction(
      {
        authority: authority.publicKey,
        gumballMachine: gumballMachineAcctKey,
      },
      args
    );
    await execute(GumballMachine.provider, [updateConfigLinesInstr], [authority]);

    const onChainGumballMachineAccount =
      await GumballMachine.provider.connection.getAccountInfo(
        gumballMachineAcctKey
      );
    const gumballMachine = decodeGumballMachine(
      onChainGumballMachineAccount.data,
      gumballMachineAcctSize
    );

    // Create the expected buffer for the indices of the account
    const expectedIndexArrBuffer = [
      ...Array(gumballMachineAcctConfigIndexArrSize / 4).keys(),
    ].reduce(
      (prevVal, curVal) =>
        Buffer.concat([prevVal, Buffer.from(num32ToBuffer(curVal))]),
      Buffer.from([])
    );
    assertGumballMachineConfigProperties(
      gumballMachine,
      expectedIndexArrBuffer,
      allExpectedInitializedConfigLines,
      gumballMachineAcctConfigLinesSize
    );
  }

  async function updateHeaderMetadata(
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    gumballMachineAcctSize,
    newHeader: UpdateHeaderMetadataInstructionArgs,
    resultingExpectedOnChainHeader: GumballMachineHeader
  ) {
    const updateHeaderMetadataInstr = createUpdateHeaderMetadataInstruction(
      {
        gumballMachine: gumballMachineAcctKey,
        authority: authority.publicKey,
      },
      newHeader
    );
    await execute(GumballMachine.provider, [updateHeaderMetadataInstr], [authority]);

    const onChainGumballMachineAccount =
      await GumballMachine.provider.connection.getAccountInfo(
        gumballMachineAcctKey
      );
    const gumballMachine = decodeGumballMachine(
      onChainGumballMachineAccount.data,
      gumballMachineAcctSize
    );
    assertGumballMachineHeaderProperties(
      gumballMachine,
      resultingExpectedOnChainHeader
    );
  }

  async function dispenseCompressedNFTForSol(
    numNFTs: number,
    payer: Keypair,
    receiver: PublicKey,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    verbose?: boolean
  ) {
    const requestCU = ComputeBudgetProgram.requestUnits({
      units: 1.4e6,
      additionalFee: 0,
    });
    const dispenseInstr = await createDispenseNFTForSolIx(
      { numItems: numNFTs },
      payer.publicKey,
      receiver,
      gumballMachineAcctKeypair.publicKey,
      merkleRollKeypair.publicKey
    );
    await execute(
      GumballMachine.provider,
      [requestCU, dispenseInstr],
      [payer],
      true,
      verbose
    );
  }

  async function dispenseCompressedNFTForTokens(
    numNFTs: number,
    payer: Keypair,
    payerTokens: PublicKey,
    receiver: PublicKey,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    verbose?: boolean
  ) {
    const dispenseInstr = await createDispenseNFTForTokensIx(
      { numItems: numNFTs },
      payer.publicKey,
      payerTokens,
      receiver,
      gumballMachineAcctKeypair.publicKey,
      merkleRollKeypair.publicKey
    );
    await execute(GumballMachine.provider, [dispenseInstr], [payer], true, verbose);
  }

  async function destroyGumballMachine(
    gumballMachineAcctKeypair: Keypair,
    authorityKeypair: Keypair
  ) {
    const originalGumballMachineAcctBalance = await connection.getBalance(
      gumballMachineAcctKeypair.publicKey
    );
    const originalAuthorityAcctBalance = await connection.getBalance(
      authorityKeypair.publicKey
    );
    const destroyInstr = createDestroyInstruction({
      gumballMachine: gumballMachineAcctKeypair.publicKey,
      authority: authorityKeypair.publicKey,
    });
    await execute(GumballMachine.provider, [destroyInstr], [authorityKeypair]);

    assert(
      0 === (await connection.getBalance(gumballMachineAcctKeypair.publicKey)),
      "Failed to remove lamports from gumball machine acct"
    );

    const expectedAuthorityAcctBalance =
      originalAuthorityAcctBalance + originalGumballMachineAcctBalance;
    assert(
      expectedAuthorityAcctBalance ===
      (await connection.getBalance(authorityKeypair.publicKey)),
      "Failed to transfer correct balance to authority"
    );
  }

  before(async function () {
    payer = Keypair.generate();
    connection = new web3Connection("http://localhost:8899", {
      commitment: "confirmed",
    });
    wallet = new NodeWallet(payer);
    anchor.setProvider(
      new AnchorProvider(connection, wallet, {
        commitment: connection.commitment,
        skipPreflight: true,
      })
    );
    GumballMachine = anchor.workspace.GumballMachine as Program<GumballMachine>;
    Bubblegum = anchor.workspace.Bubblegum as Program<Bubblegum>;
    GummyrollProgramId = anchor.workspace.Gummyroll.programId;
    BubblegumProgramId = anchor.workspace.Bubblegum.programId;

    // Give funds to the payer for the whole suite
    await GumballMachine.provider.connection.confirmTransaction(
      await GumballMachine.provider.connection.requestAirdrop(
        payer.publicKey,
        200e9
      ),
      "confirmed"
    );
  });

  describe("native sol project with config lines", function () {
    let GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE;
    let GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE;
    let GUMBALL_MACHINE_ACCT_SIZE;
    let MERKLE_ROLL_ACCT_SIZE;

    let creatorPaymentWallet: Keypair;
    let exampleAdditionalSecondarySaleRoyaltyRecipient: Keypair;

    beforeEach(async function () {
      creatorAddress = Keypair.generate();
      creatorPaymentWallet = Keypair.generate();
      nftBuyer = Keypair.generate();
      gumballMachineAcctKeypair = Keypair.generate();
      merkleRollKeypair = Keypair.generate();
      exampleAdditionalSecondarySaleRoyaltyRecipient = Keypair.generate();
      creatorKeys = [
        creatorPaymentWallet.publicKey,
        exampleAdditionalSecondarySaleRoyaltyRecipient.publicKey,
      ];
      creatorShares = Uint8Array.from([10, 90]);

      baseGumballMachineInitProps = {
        maxDepth: 3,
        maxBufferSize: 8,
        urlBase: strToByteArray("https://arweave.net", 64),
        nameBase: strToByteArray("GUMBALL", 32),
        symbol: strToByteArray("GUMBALL", 8),
        encodeMethod: EncodeMethod.UTF8,
        sellerFeeBasisPoints: 100,
        isMutable: true,
        retainAuthority: true,
        price: new BN(10),
        goLiveDate: new BN(1234.0),
        botWallet: Keypair.generate().publicKey,
        receiver: creatorPaymentWallet.publicKey,
        authority: creatorAddress.publicKey,
        collectionKey: SystemProgram.programId, // 0x0 -> no collection key
        extensionLen: new BN(28),
        maxMintSize: 8,
        maxItems: 8,
        creatorKeys,
        creatorShares,
      };

      GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE =
        baseGumballMachineInitProps.maxItems * 4;
      GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE =
        baseGumballMachineInitProps.maxItems *
        val(baseGumballMachineInitProps.extensionLen).toNumber();
      GUMBALL_MACHINE_ACCT_SIZE =
        gumballMachineHeaderBeet.byteSize +
        GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE +
        GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE;
      MERKLE_ROLL_ACCT_SIZE = getMerkleRollAccountSize(
        baseGumballMachineInitProps.maxDepth,
        baseGumballMachineInitProps.maxBufferSize
      );

      // Give creator enough funds to produce accounts for NFT
      await GumballMachine.provider.connection.confirmTransaction(
        await GumballMachine.provider.connection.requestAirdrop(
          creatorAddress.publicKey,
          LAMPORTS_PER_SOL
        ),
        "confirmed"
      );

      await initializeGumballMachine(
        creatorAddress,
        gumballMachineAcctKeypair,
        GUMBALL_MACHINE_ACCT_SIZE,
        merkleRollKeypair,
        MERKLE_ROLL_ACCT_SIZE,
        baseGumballMachineInitProps,
        NATIVE_MINT
      );

      await initializeIndicesAndAssert(
        baseGumballMachineInitProps.maxItems,
        creatorAddress,
        gumballMachineAcctKeypair.publicKey,
        GUMBALL_MACHINE_ACCT_SIZE
      );

      await addConfigLines(
        creatorAddress,
        gumballMachineAcctKeypair.publicKey,
        GUMBALL_MACHINE_ACCT_SIZE,
        GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE,
        GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE,
        strToByteUint8Array("uluvnpwncgchwnbqfpbtdlcpdthc"),
        Buffer.from("uluvnpwncgchwnbqfpbtdlcpdthc")
      );
    });

    describe("dispense nft sol instruction", function () {
      beforeEach(async function () {
        // Give the recipient address enough money to not get rent exempt
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(
            baseGumballMachineInitProps.receiver,
            LAMPORTS_PER_SOL
          ),
          "confirmed"
        );

        // Fund the NFT Buyer
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(
            nftBuyer.publicKey,
            LAMPORTS_PER_SOL
          ),
          "confirmed"
        );
      });
      describe("transaction atomicity attacks fail", function () {
        let dispenseNFTForSolInstr;
        let dummyNewAcctKeypair;
        let dummyInstr;

        beforeEach(async function () {
          dispenseNFTForSolInstr = await createDispenseNFTForSolIx(
            { numItems: 1 },
            nftBuyer.publicKey,
            baseGumballMachineInitProps.receiver,
            gumballMachineAcctKeypair.publicKey,
            merkleRollKeypair.publicKey
          );
          dummyNewAcctKeypair = Keypair.generate();
          dummyInstr = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: dummyNewAcctKeypair.publicKey,
            lamports: 10000000,
            space: 100,
            programId: GumballMachine.programId,
          });
        });

        it("Cannot dispense NFT for SOL with subsequent instructions in transaction", async function () {
          let confirmedTxId: string;
          try {
            confirmedTxId = await execute(GumballMachine.provider, [dispenseNFTForSolInstr, dummyInstr], [nftBuyer, payer, dummyNewAcctKeypair]);
          } catch (e) { }

          if (confirmedTxId)
            assert(
              false,
              "Dispense should fail when part of transaction with multiple instructions, but it succeeded"
            );
        });

        it("Cannot dispense NFT for SOL with prior instructions in transaction", async function () {
          let confirmedTxId: string;
          try {
            confirmedTxId = await execute(GumballMachine.provider, [dummyInstr, dispenseNFTForSolInstr], [nftBuyer, payer, dummyNewAcctKeypair]);
          } catch (e) { }

          if (confirmedTxId)
            assert(
              false,
              "Dispense should fail when part of transaction with multiple instructions, but it succeeded"
            );
        });
      });

      it("Can dispense single NFT paid in sol", async function () {
        // Give the recipient address enough money to not get rent exempt
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(
            baseGumballMachineInitProps.receiver,
            LAMPORTS_PER_SOL
          ),
          "confirmed"
        );

        // Fund the NFT Buyer
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(
            nftBuyer.publicKey,
            LAMPORTS_PER_SOL
          ),
          "confirmed"
        );

        const nftBuyerBalanceBeforePurchase = await connection.getBalance(
          nftBuyer.publicKey
        );
        const creatorBalanceBeforePurchase = await connection.getBalance(
          baseGumballMachineInitProps.receiver
        );

        // Purchase the compressed NFT with SOL
        await dispenseCompressedNFTForSol(
          1,
          nftBuyer,
          baseGumballMachineInitProps.receiver,
          gumballMachineAcctKeypair,
          merkleRollKeypair
        );
        const creatorBalanceAfterPurchase = await connection.getBalance(
          baseGumballMachineInitProps.receiver,
          "confirmed"
        );

        // Assert on how the creator and buyer's balances changed
        assert(
          creatorBalanceAfterPurchase ===
          creatorBalanceBeforePurchase +
          val(baseGumballMachineInitProps.price).toNumber(),
          "Creator balance did not update as expected after NFT purchase"
        );

        const nftBuyerBalanceAfterPurchase = await connection.getBalance(
          nftBuyer.publicKey,
          "confirmed"
        );

        assert(
          nftBuyerBalanceAfterPurchase ===
          nftBuyerBalanceBeforePurchase -
          val(baseGumballMachineInitProps.price).toNumber(),
          "NFT purchaser balance did not decrease as expected after NFT purchase"
        );
      });
    });
    // @notice: We only test admin instructions on SOL projects because they are completely (for now) independent of project mint
    describe("admin instructions", function () {
      it("Can update config lines", async function () {
        await updateConfigLines(
          creatorAddress,
          gumballMachineAcctKeypair.publicKey,
          GUMBALL_MACHINE_ACCT_SIZE,
          GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE,
          GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE,
          Buffer.from("aaavnpwncgchwnbqfpbtdlcpdaaa"),
          Buffer.from("aaavnpwncgchwnbqfpbtdlcpdaaa"),
          new BN(0)
        );
      });
      it("Can update gumball header", async function () {
        const newGumballMachineHeader: UpdateHeaderMetadataInstructionArgs = {
          urlBase: strToByteArray("https://arweave.net", 64),
          nameBase: strToByteArray("GUMBALL", 32),
          symbol: strToByteArray("GUMBALL", 8),
          encodeMethod: EncodeMethod.Base58Encode,
          sellerFeeBasisPoints: 50,
          isMutable: false,
          retainAuthority: false,
          price: new BN(100),
          goLiveDate: new BN(5678.0),
          botWallet: Keypair.generate().publicKey,
          authority: Keypair.generate().publicKey,
          receiver: Keypair.generate().publicKey,
          maxMintSize: 5,
          creatorKeys: [],
          creatorShares: Uint8Array.from([]),
        };

        const expectedOnChainHeader: GumballMachineHeader = {
          urlBase: newGumballMachineHeader.urlBase,
          nameBase: newGumballMachineHeader.nameBase,
          symbol: newGumballMachineHeader.symbol,
          configLineEncodeMethod: 1,
          sellerFeeBasisPoints: newGumballMachineHeader.sellerFeeBasisPoints,
          isMutable: newGumballMachineHeader.isMutable ? 1 : 0,
          retainAuthority: newGumballMachineHeader.retainAuthority ? 1 : 0,
          creators: [],
          price: newGumballMachineHeader.price,
          goLiveDate: newGumballMachineHeader.goLiveDate,
          mint: NATIVE_MINT,
          botWallet: newGumballMachineHeader.botWallet,
          receiver: newGumballMachineHeader.receiver,
          authority: newGumballMachineHeader.authority,
          collectionKey: baseGumballMachineInitProps.collectionKey,
          extensionLen: baseGumballMachineInitProps.extensionLen,
          maxMintSize: newGumballMachineHeader.maxMintSize,
          remaining: 0,
          maxItems: baseGumballMachineInitProps.maxItems,
          totalItemsAdded: 0,
          smallestUninitializedIndex: baseGumballMachineInitProps.maxItems,
          padding: [0, 0, 0, 0, 0, 0, 0]
        };
        await updateHeaderMetadata(
          creatorAddress,
          gumballMachineAcctKeypair.publicKey,
          GUMBALL_MACHINE_ACCT_SIZE,
          newGumballMachineHeader,
          expectedOnChainHeader
        );
      });
      it("Can destroy gumball machine and reclaim lamports", async function () {
        await destroyGumballMachine(
          gumballMachineAcctKeypair,
          creatorAddress
        );
      });
    });
  });
  describe("spl token projects with config lines", function () {
    let GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE;
    let GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE;
    let GUMBALL_MACHINE_ACCT_SIZE;
    let MERKLE_ROLL_ACCT_SIZE;

    let someMint: PublicKey;
    let creatorReceiverTokenAccount;
    let botWallet;
    let nftBuyerTokenAccount;

    beforeEach(async function () {
      creatorAddress = Keypair.generate();
      gumballMachineAcctKeypair = Keypair.generate();
      merkleRollKeypair = Keypair.generate();
      nftBuyer = Keypair.generate();
      botWallet = Keypair.generate();

      // Give creator enough funds to produce accounts for gumball-machine
      await GumballMachine.provider.connection.confirmTransaction(
        await GumballMachine.provider.connection.requestAirdrop(
          creatorAddress.publicKey,
          4 * LAMPORTS_PER_SOL
        ),
        "confirmed"
      );

      someMint = await createMint(
        connection,
        payer,
        payer.publicKey,
        null,
        9
      );
      creatorReceiverTokenAccount = await getOrCreateAssociatedTokenAccount(
        connection,
        payer,
        someMint,
        creatorAddress.publicKey
      );

      creatorKeys = [creatorReceiverTokenAccount.address];
      creatorShares = Uint8Array.from([100]);

      baseGumballMachineInitProps = {
        maxDepth: 3,
        maxBufferSize: 8,
        urlBase: strToByteArray("https://arweave.net", 64),
        nameBase: strToByteArray("GUMBALL", 32),
        symbol: strToByteArray("GUMBALL", 8),
        sellerFeeBasisPoints: 100,
        isMutable: true,
        retainAuthority: true,
        encodeMethod: EncodeMethod.Base58Encode,
        price: new BN(10),
        goLiveDate: new BN(1234.0),
        botWallet: botWallet.publicKey,
        receiver: creatorReceiverTokenAccount.address,
        authority: creatorAddress.publicKey,
        collectionKey: SystemProgram.programId, // 0x0 -> no collection key
        extensionLen: new BN(28),
        maxMintSize: 8,
        maxItems: 8,
        creatorKeys,
        creatorShares,
      };

      GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE =
        baseGumballMachineInitProps.maxItems * 4;
      GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE =
        baseGumballMachineInitProps.maxItems *
        val(baseGumballMachineInitProps.extensionLen).toNumber();
      GUMBALL_MACHINE_ACCT_SIZE =
        gumballMachineHeaderBeet.byteSize +
        GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE +
        GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE;
      MERKLE_ROLL_ACCT_SIZE = getMerkleRollAccountSize(
        baseGumballMachineInitProps.maxDepth,
        baseGumballMachineInitProps.maxBufferSize
      );

      await initializeGumballMachine(
        creatorAddress,
        gumballMachineAcctKeypair,
        GUMBALL_MACHINE_ACCT_SIZE,
        merkleRollKeypair,
        MERKLE_ROLL_ACCT_SIZE,
        baseGumballMachineInitProps,
        someMint
      );

      await initializeIndicesAndAssert(
        baseGumballMachineInitProps.maxItems,
        creatorAddress,
        gumballMachineAcctKeypair.publicKey,
        GUMBALL_MACHINE_ACCT_SIZE
      );

      await addConfigLines(
        creatorAddress,
        gumballMachineAcctKeypair.publicKey,
        GUMBALL_MACHINE_ACCT_SIZE,
        GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE,
        GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE,
        Buffer.from(
          "uluvnpwncgchwnbqfpbtdlcpdthc" + "aauvnpwncgchwnbqfpbtdlcpdthc"
        ),
        Buffer.from(
          "uluvnpwncgchwnbqfpbtdlcpdthc" + "aauvnpwncgchwnbqfpbtdlcpdthc"
        )
      );

      // Create and fund the NFT pruchaser
      nftBuyerTokenAccount = await getOrCreateAssociatedTokenAccount(
        connection,
        payer,
        someMint,
        nftBuyer.publicKey
      );
      await mintTo(
        connection,
        payer,
        someMint,
        nftBuyerTokenAccount.address,
        payer,
        50
      );
    });
    describe("transaction atomicity attacks fail", function () {
      let dispenseNFTForTokensInstr;
      let dummyNewAcctKeypair;
      let dummyInstr;

      beforeEach(async function () {
        dispenseNFTForTokensInstr = await createDispenseNFTForTokensIx(
          { numItems: 1 },
          nftBuyer.publicKey,
          nftBuyerTokenAccount.address,
          creatorReceiverTokenAccount.address,
          gumballMachineAcctKeypair.publicKey,
          merkleRollKeypair.publicKey
        );
        dummyNewAcctKeypair = Keypair.generate();
        dummyInstr = SystemProgram.createAccount({
          fromPubkey: payer.publicKey,
          newAccountPubkey: dummyNewAcctKeypair.publicKey,
          lamports: 10000000,
          space: 100,
          programId: GumballMachine.programId,
        });
      });

      it("Cannot dispense NFT for tokens with subsequent instructions in transaction", async function () {
        try {
          await execute(GumballMachine.provider, [dispenseNFTForTokensInstr, dummyInstr], [nftBuyer, payer, dummyNewAcctKeypair]);
          assert(
            false,
            "Dispense should fail when part of transaction with multiple instructions, but it succeeded"
          );
        } catch (e) { }
      });

      it("Cannot dispense NFT for SOL with prior instructions in transaction", async function () {
        try {
          await execute(GumballMachine.provider, [dummyInstr, dispenseNFTForTokensInstr], [nftBuyer, payer, dummyNewAcctKeypair]);
          assert(
            false,
            "Dispense should fail when part of transaction with multiple instructions, but it succeeded"
          );
        } catch (e) { }
      });
    });

    it("Can dispense multiple NFTs paid in token, but not more than remaining, unminted config lines", async function () {
      let buyerTokenAccount = await getAccount(
        connection,
        nftBuyerTokenAccount.address
      );
      await dispenseCompressedNFTForTokens(
        3,
        nftBuyer,
        nftBuyerTokenAccount.address,
        creatorReceiverTokenAccount.address,
        gumballMachineAcctKeypair,
        merkleRollKeypair
      );

      let newCreatorTokenAccount = await getAccount(
        connection,
        creatorReceiverTokenAccount.address
      );
      let newBuyerTokenAccount = await getAccount(
        connection,
        nftBuyerTokenAccount.address
      );

      // Since there were only two config lines added, we should have only successfully minted (and paid for) two NFTs
      const newExpectedCreatorTokenBalance =
        Number(creatorReceiverTokenAccount.amount) +
        val(baseGumballMachineInitProps.price).toNumber() * 2;
      assert(
        Number(newCreatorTokenAccount.amount) ===
        newExpectedCreatorTokenBalance,
        "The creator did not receive their payment as expected"
      );

      const newExpectedBuyerTokenBalance =
        Number(buyerTokenAccount.amount) -
        val(baseGumballMachineInitProps.price).toNumber() * 2;
      assert(
        Number(newBuyerTokenAccount.amount) === newExpectedBuyerTokenBalance,
        "The nft buyer did not pay for the nft as expected"
      );

      // Should not be able to dispense without any NFTs remaining
      try {
        await dispenseCompressedNFTForTokens(
          1,
          nftBuyer,
          nftBuyerTokenAccount.address,
          creatorReceiverTokenAccount.address,
          gumballMachineAcctKeypair,
          merkleRollKeypair
        );
        assert(
          false,
          "Dispense unexpectedly succeeded with no NFTs remaining"
        );
      } catch (e) { }
    });
  });

  describe("native sol project without config lines", function () {
    let GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE;
    let GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE;
    let GUMBALL_MACHINE_ACCT_SIZE;
    let MERKLE_ROLL_ACCT_SIZE;
    let creatorPaymentWallet: Keypair;
    let exampleAdditionalSecondarySaleRoyaltyRecipient: Keypair;

    beforeEach(async function () {
      creatorAddress = Keypair.generate();
      creatorPaymentWallet = Keypair.generate();
      nftBuyer = Keypair.generate();
      gumballMachineAcctKeypair = Keypair.generate();
      merkleRollKeypair = Keypair.generate();
      exampleAdditionalSecondarySaleRoyaltyRecipient = Keypair.generate();
      creatorKeys = [
        creatorPaymentWallet.publicKey,
        exampleAdditionalSecondarySaleRoyaltyRecipient.publicKey,
      ];
      creatorShares = Uint8Array.from([10, 90]);

      baseGumballMachineInitProps = {
        maxDepth: 5,
        maxBufferSize: 8,
        urlBase: strToByteArray("https://arweave.net", 64),
        nameBase: strToByteArray("GUMBALL", 32),
        symbol: strToByteArray("GUMBALL", 8),
        encodeMethod: EncodeMethod.UTF8,
        sellerFeeBasisPoints: 100,
        isMutable: true,
        retainAuthority: true,
        price: new BN(10),
        goLiveDate: new BN(1234.0),
        botWallet: Keypair.generate().publicKey,
        receiver: creatorPaymentWallet.publicKey,
        authority: creatorAddress.publicKey,
        collectionKey: SystemProgram.programId, // 0x0 -> no collection key
        extensionLen: new BN(0),
        maxMintSize: 10,
        maxItems: 32,
        creatorKeys,
        creatorShares,
      };

      GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE =
        baseGumballMachineInitProps.maxItems * 4;
      GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE = 0;
      GUMBALL_MACHINE_ACCT_SIZE =
        gumballMachineHeaderBeet.byteSize +
        GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE +
        GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE;
      MERKLE_ROLL_ACCT_SIZE = getMerkleRollAccountSize(5, 8, 2);

      // Give creator enough funds to produce accounts for NFT
      await GumballMachine.provider.connection.confirmTransaction(
        await GumballMachine.provider.connection.requestAirdrop(
          creatorAddress.publicKey,
          50 * LAMPORTS_PER_SOL
        ),
        "confirmed"
      );

      await initializeGumballMachine(
        creatorAddress,
        gumballMachineAcctKeypair,
        GUMBALL_MACHINE_ACCT_SIZE,
        merkleRollKeypair,
        MERKLE_ROLL_ACCT_SIZE,
        baseGumballMachineInitProps,
        NATIVE_MINT
      );

      await initializeIndicesAndAssert(
        baseGumballMachineInitProps.maxItems,
        creatorAddress,
        gumballMachineAcctKeypair.publicKey,
        GUMBALL_MACHINE_ACCT_SIZE
      );
    });

    describe("dispense nft sol instruction", function () {

      beforeEach(async function () {
        // Give the recipient address enough money to not get rent exempt
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(
            baseGumballMachineInitProps.receiver,
            LAMPORTS_PER_SOL
          ),
          "confirmed"
        );

        // Fund the NFT Buyer
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(
            nftBuyer.publicKey,
            LAMPORTS_PER_SOL
          ),
          "confirmed"
        );
      });
      it("Can dispense NFTs paid in sol", async function () {
        // Give the recipient address enough money to not get rent exempt
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(
            baseGumballMachineInitProps.receiver,
            LAMPORTS_PER_SOL
          ),
          "confirmed"
        );

        // Fund the NFT Buyer
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(
            nftBuyer.publicKey,
            LAMPORTS_PER_SOL
          ),
          "confirmed"
        );

        // Purchase compressed NFT with SOL
        await dispenseCompressedNFTForSol(
          4,
          nftBuyer,
          baseGumballMachineInitProps.receiver,
          gumballMachineAcctKeypair,
          merkleRollKeypair
        );
      });
    });
    describe("admin instructions", function () {
      it("Can update gumball header", async function () {
        const newCreatorKeys = [
          Keypair.generate().publicKey,
          Keypair.generate().publicKey,
          Keypair.generate().publicKey,
        ];
        const newCreatorShares = Uint8Array.from([50, 25, 25]);

        const newGumballMachineHeader: UpdateHeaderMetadataInstructionArgs = {
          urlBase: strToByteArray("https://arweave.net", 64),
          nameBase: strToByteArray("GUMBALL", 32),
          symbol: strToByteArray("GUMBALL", 8),
          encodeMethod: EncodeMethod.Base58Encode,
          sellerFeeBasisPoints: 50,
          isMutable: false,
          retainAuthority: false,
          price: new BN(100),
          goLiveDate: new BN(5678.0),
          botWallet: Keypair.generate().publicKey,
          authority: Keypair.generate().publicKey,
          receiver: Keypair.generate().publicKey,
          maxMintSize: 15,
          creatorKeys: newCreatorKeys,
          creatorShares: newCreatorShares,
        };

        let expectedCreators = [];
        for (let i = 0; i < newCreatorKeys.length; i++) {
          let c: GumballCreatorAdapter = {
            address: newCreatorKeys[i],
            share: newCreatorShares[i],
            verified: 0,
          };
          expectedCreators.push(c);
        }
        const expectedOnChainHeader: GumballMachineHeader = {
          urlBase: newGumballMachineHeader.urlBase,
          nameBase: newGumballMachineHeader.nameBase,
          symbol: newGumballMachineHeader.symbol,
          configLineEncodeMethod: 1,
          sellerFeeBasisPoints: newGumballMachineHeader.sellerFeeBasisPoints,
          isMutable: newGumballMachineHeader.isMutable ? 1 : 0,
          retainAuthority: newGumballMachineHeader.retainAuthority ? 1 : 0,
          creators: expectedCreators,
          price: newGumballMachineHeader.price,
          goLiveDate: newGumballMachineHeader.goLiveDate,
          mint: NATIVE_MINT,
          botWallet: newGumballMachineHeader.botWallet,
          receiver: newGumballMachineHeader.receiver,
          authority: newGumballMachineHeader.authority,
          collectionKey: baseGumballMachineInitProps.collectionKey,
          extensionLen: baseGumballMachineInitProps.extensionLen,
          maxMintSize: newGumballMachineHeader.maxMintSize,
          remaining: 0,
          maxItems: baseGumballMachineInitProps.maxItems,
          totalItemsAdded: 0,
          smallestUninitializedIndex: baseGumballMachineInitProps.maxItems,
          padding: [0, 0, 0, 0, 0, 0, 0]
        };
        await updateHeaderMetadata(
          creatorAddress,
          gumballMachineAcctKeypair.publicKey,
          GUMBALL_MACHINE_ACCT_SIZE,
          newGumballMachineHeader,
          expectedOnChainHeader
        );
      });
      it("Can destroy gumball machine and reclaim lamports", async function () {
        await destroyGumballMachine(
          gumballMachineAcctKeypair,
          creatorAddress
        );
      });
    });
  });
});
