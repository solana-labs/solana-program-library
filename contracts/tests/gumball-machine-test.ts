import * as anchor from "@project-serum/anchor";
import { keccak_256 } from "js-sha3";
import { BN, Provider, Program } from "@project-serum/anchor";
import { Bubblegum } from "../target/types/bubblegum";
import { Gummyroll } from "../target/types/gummyroll";
import { Key, PROGRAM_ID } from "@metaplex-foundation/mpl-token-metadata";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_SLOT_HASHES_PUBKEY,
  LAMPORTS_PER_SOL,
  TransactionInstruction,
} from "@solana/web3.js";
import { assert } from "chai";

import { buildTree, Tree } from "./merkle-tree";
import {
  decodeMerkleRoll,
  getMerkleRollAccountSize,
  assertOnChainMerkleRollProperties
} from "../sdk/gummyroll";
import {
  GUMBALL_MACHINE_HEADER_SIZE,
  GumballMachine,
  InitGumballMachineProps,
  decodeGumballMachine,
  OnChainGumballMachine,
  createDispenseNFTForSolIx,
  createDispenseNFTForTokensIx,
  createUpdateConfigLinesIx,
  createDestroyGumballMachineIx,
  createInitializeGumballMachineIxs,
  createUpdateHeaderMetadataIx,
  getBubblegumAuthorityPDAKey,
} from '../sdk/gumball-machine';
import {
  InitializeGumballMachineInstructionArgs,
  createInitializeGumballMachineInstruction,
  createAddConfigLinesInstruction,
  createUpdateConfigLinesInstruction,
  UpdateHeaderMetadataInstructionArgs,
  UpdateConfigLinesInstructionArgs,
  createUpdateHeaderMetadataInstruction,
  createDestroyInstruction,
  createDispenseNftSolInstruction,
  createDispenseNftTokenInstruction
} from "../sdk/gumball-machine/src/generated/instructions";
import {
  val,
  strToByteArray,
  strToByteUint8Array
} from "../sdk/utils/index";
import {
  GumballMachineHeader
} from "../sdk/gumball-machine/src/generated/types/index";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { getAssociatedTokenAddress, createMint, getOrCreateAssociatedTokenAccount, mintTo, getAccount } from "../../deps/solana-program-library/token/js/src";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  NATIVE_MINT
} from "@solana/spl-token";
import { logTx, num32ToBuffer } from "./utils";

// @ts-ignore
let GumballMachine;
let Bubblegum;
// @ts-ignore
let GummyrollProgramId;
let BubblegumProgramId;

// TODO(sorend): port this to utils
function arrayEquals(a, b) {
  return Array.isArray(a) &&
      Array.isArray(b) &&
      a.length === b.length &&
      a.every((val, index) => val === b[index]);
}

describe("gumball-machine", () => {
  // Configure the client to use the local cluster.

  const payer = Keypair.generate();
  
  let connection = new web3Connection("http://localhost:8899", {
    commitment: "confirmed",
  });

  let wallet = new NodeWallet(payer);
  anchor.setProvider(
    new Provider(connection, wallet, {
      commitment: connection.commitment,
      skipPreflight: true,
    })
  );

  GumballMachine = anchor.workspace.GumballMachine as Program<GumballMachine>;
  Bubblegum = anchor.workspace.Bubblegum as Program<Bubblegum>;
  GummyrollProgramId = anchor.workspace.Gummyroll.programId;
  BubblegumProgramId = anchor.workspace.Bubblegum.programId;
  
  function assertGumballMachineHeaderProperties(gm: OnChainGumballMachine, expectedHeader: GumballMachineHeader) {
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
      gm.header.creatorAddress.equals(expectedHeader.creatorAddress),
      "Gumball Machine set with incorrect creatorAddress"
    );
    assert(
      val(gm.header.extensionLen).eq(val(expectedHeader.extensionLen)),
      "Gumball Machine has incorrect extensionLen"
    );
    assert(
      val(gm.header.maxMintSize).eq(val(expectedHeader.maxMintSize)),
      "Gumball Machine has incorrect maxMintSize"
    );
    assert(
      val(gm.header.maxItems).eq(val(expectedHeader.maxItems)),
      "Gumball Machine has incorrect max items"
    );

    // TODO(sorend): add assertion on mint
  }

  function assertGumballMachineConfigProperties(gm: OnChainGumballMachine, expectedIndexArray: Buffer, expectedConfigLines: Buffer, onChainConfigLinesNumBytes: number) {
    assert(
      gm.configData.indexArray.equals(expectedIndexArray),
      "Onchain index array doesn't match expectation"
    )

    // Calculate full-sized on-chain config bytes buffer, we must null pad the buffer up to the end of the account size
    const numExpectedInitializedBytesInConfig = expectedConfigLines.byteLength
    const bufferOfNonInitializedConfigLineBytes = Buffer.from("\0".repeat(onChainConfigLinesNumBytes-numExpectedInitializedBytesInConfig))
    const actualExpectedConfigLinesBuffer = Buffer.concat([expectedConfigLines, bufferOfNonInitializedConfigLineBytes])
    assert(
      gm.configData.configLines.equals(actualExpectedConfigLinesBuffer),
      "Config lines on gumball machine do not match expectation"
    )
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
    const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDAKey(merkleRollKeypair.publicKey, BubblegumProgramId);
    //const initializeGumballMachineInstr = await createInitializeGumballMachineInstruction()
    const initializeGumballMachineInstrs = await createInitializeGumballMachineIxs(payer, gumballMachineAcctKeypair, gumballMachineAcctSize, merkleRollKeypair, merkleRollAccountSize, gumballMachineInitArgs, mint, GummyrollProgramId, BubblegumProgramId, GumballMachine);
    const tx = new Transaction();
    initializeGumballMachineInstrs.forEach((instr) => tx.add(instr));
    await GumballMachine.provider.send(tx, [payer, gumballMachineAcctKeypair, merkleRollKeypair], {
      commitment: "confirmed",
    });

    const tree = buildTree(Array(2 ** gumballMachineInitArgs.maxDepth).fill(Buffer.alloc(32)));
    await assertOnChainMerkleRollProperties(GumballMachine.provider.connection, gumballMachineInitArgs.maxDepth, gumballMachineInitArgs.maxBufferSize, bubblegumAuthorityPDAKey, new PublicKey(tree.root), merkleRollKeypair.publicKey);

    const onChainGumballMachineAccount = await GumballMachine.provider.connection.getAccountInfo(
      gumballMachineAcctKeypair.publicKey
    );

    const gumballMachine = decodeGumballMachine(onChainGumballMachineAccount.data, gumballMachineAcctSize);
    
    let expectedOnChainHeader: GumballMachineHeader = {
      urlBase: gumballMachineInitArgs.urlBase,
      nameBase: gumballMachineInitArgs.nameBase,
      symbol: gumballMachineInitArgs.symbol, 
      sellerFeeBasisPoints: gumballMachineInitArgs.sellerFeeBasisPoints,
      isMutable: gumballMachineInitArgs.isMutable ? 1 : 0,
      retainAuthority: gumballMachineInitArgs.retainAuthority ? 1 : 0,
      padding: [0,0,0,0],
      price: gumballMachineInitArgs.price,
      goLiveDate: gumballMachineInitArgs.goLiveDate,
      mint,
      botWallet: gumballMachineInitArgs.botWallet,
      receiver: gumballMachineInitArgs.receiver,
      authority: gumballMachineInitArgs.authority,
      collectionKey: gumballMachineInitArgs.collectionKey, // 0x0 -> no collection key
      creatorAddress: payer.publicKey,
      extensionLen: gumballMachineInitArgs.extensionLen,
      maxMintSize: gumballMachineInitArgs.maxMintSize,
      remaining: new BN(0),
      maxItems: gumballMachineInitArgs.maxItems,
      totalItemsAdded: new BN(0)
    }
    assertGumballMachineHeaderProperties(gumballMachine, expectedOnChainHeader);
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
        authority: authority.publicKey
      },
      {
        newConfigLinesData: configLinesToAdd
      }
    );
    const tx = new Transaction().add(addConfigLinesInstr)
    await GumballMachine.provider.send(tx, [authority], {
      commitment: "confirmed",
    });
    const onChainGumballMachineAccount = await GumballMachine.provider.connection.getAccountInfo(
      gumballMachineAcctKey
    );
    const gumballMachine = decodeGumballMachine(onChainGumballMachineAccount.data, gumballMachineAcctSize);

    // Create the expected buffer for the indices of the account
    const expectedIndexArrBuffer = [...Array(gumballMachineAcctConfigIndexArrSize/4).keys()].reduce(
      (prevVal, curVal) => Buffer.concat([prevVal, Buffer.from(num32ToBuffer(curVal))]),
      Buffer.from([])
    );

    assertGumballMachineConfigProperties(gumballMachine, expectedIndexArrBuffer, allExpectedInitializedConfigLines, gumballMachineAcctConfigLinesSize);
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
    const args: UpdateConfigLinesInstructionArgs = { startingLine: indexOfFirstLineToUpdate, newConfigLinesData: updatedConfigLines };
    const updateConfigLinesInstr = createUpdateConfigLinesIx(authority, gumballMachineAcctKey, args);
    const tx = new Transaction().add(updateConfigLinesInstr)
    await GumballMachine.provider.send(tx, [authority], {
      commitment: "confirmed",
    });

    const onChainGumballMachineAccount = await GumballMachine.provider.connection.getAccountInfo(
      gumballMachineAcctKey
    );
    const gumballMachine = decodeGumballMachine(onChainGumballMachineAccount.data, gumballMachineAcctSize);
    
    // Create the expected buffer for the indices of the account
    const expectedIndexArrBuffer = [...Array(gumballMachineAcctConfigIndexArrSize/4).keys()].reduce(
      (prevVal, curVal) => Buffer.concat([prevVal, Buffer.from(num32ToBuffer(curVal))]),
      Buffer.from([])
    )
    assertGumballMachineConfigProperties(gumballMachine, expectedIndexArrBuffer, allExpectedInitializedConfigLines, gumballMachineAcctConfigLinesSize);
  }

  async function updateHeaderMetadata(
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    gumballMachineAcctSize,
    newHeader: UpdateHeaderMetadataInstructionArgs,
    resultingExpectedOnChainHeader: GumballMachineHeader
  ) {
    const updateHeaderMetadataInstr = createUpdateHeaderMetadataIx(authority, gumballMachineAcctKey, newHeader);
    const tx = new Transaction().add(updateHeaderMetadataInstr)
    await GumballMachine.provider.send(tx, [authority], {
      commitment: "confirmed",
    });
    const onChainGumballMachineAccount = await GumballMachine.provider.connection.getAccountInfo(
      gumballMachineAcctKey
    );
    const gumballMachine = decodeGumballMachine(onChainGumballMachineAccount.data, gumballMachineAcctSize);
    assertGumballMachineHeaderProperties(gumballMachine, resultingExpectedOnChainHeader);
  }

  async function dispenseCompressedNFTForSol(
    numNFTs: BN,
    payer: Keypair,
    receiver: PublicKey,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    noncePDAKey: PublicKey
  ) {
    const dispenseInstr = await createDispenseNFTForSolIx(
      numNFTs,
      payer,
      receiver,
      gumballMachineAcctKeypair,
      merkleRollKeypair,
      noncePDAKey,
      GummyrollProgramId,
      BubblegumProgramId,
      GumballMachine
    );
    const tx = new Transaction().add(dispenseInstr);
    await GumballMachine.provider.send(tx, [payer], {
      commitment: "confirmed",
    });

    // TODO(sorend): assert that the effects of the mint are as expected             
  }

  async function dispenseCompressedNFTForTokens(
    numNFTs: BN,
    payer: Keypair,
    payerTokens: PublicKey,
    receiver: PublicKey,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    noncePDAKey: PublicKey
  ) {
    const dispenseInstr = await createDispenseNFTForTokensIx(
      numNFTs,
      payer,
      payerTokens,
      receiver,
      gumballMachineAcctKeypair,
      merkleRollKeypair,
      noncePDAKey,
      GummyrollProgramId,
      BubblegumProgramId,
      GumballMachine
    );
    const tx = new Transaction().add(dispenseInstr);
    await GumballMachine.provider.send(tx, [payer], {
      commitment: "confirmed",
    });

    // TODO(sorend): assert that the effects of the mint are as expected             
  }

  async function destroyGumballMachine(
    gumballMachineAcctKeypair: Keypair,
    authorityKeypair: Keypair
  ) {
    const originalGumballMachineAcctBalance = await connection.getBalance(gumballMachineAcctKeypair.publicKey);
    const originalAuthorityAcctBalance = await connection.getBalance(authorityKeypair.publicKey);
    const destroyInstr = createDestroyGumballMachineIx(gumballMachineAcctKeypair, authorityKeypair);
    const tx = new Transaction().add(destroyInstr);
    await GumballMachine.provider.send(tx, [authorityKeypair], {
      commitment: "confirmed",
    });

    assert(
      0 === await connection.getBalance(gumballMachineAcctKeypair.publicKey),
      "Failed to remove lamports from gumball machine acct"
    );

    const expectedAuthorityAcctBalance = originalAuthorityAcctBalance + originalGumballMachineAcctBalance
    assert(
      expectedAuthorityAcctBalance === await connection.getBalance(authorityKeypair.publicKey),
      "Failed to transfer correct balance to authority"
    );
  }

  describe("Testing gumball-machine", async () => {
    let baseGumballMachineInitProps: InitializeGumballMachineInstructionArgs;
    let creatorAddress: Keypair;
    let gumballMachineAcctKeypair: Keypair;
    let merkleRollKeypair: Keypair;
    let noncePDAKey: PublicKey;
    let nftBuyer: Keypair;
    const GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE = 1000;
    const GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE = 7000;
    const GUMBALL_MACHINE_ACCT_SIZE = GUMBALL_MACHINE_HEADER_SIZE + GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE + GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE;
    const MERKLE_ROLL_ACCT_SIZE = getMerkleRollAccountSize(3,8);

    before(async () => {

      // Give funds to the payer for the whole suite
      await GumballMachine.provider.connection.confirmTransaction(
        await GumballMachine.provider.connection.requestAirdrop(payer.publicKey, 25e9),
        "confirmed"
      );

      [noncePDAKey] = await PublicKey.findProgramAddress(
        [Buffer.from("bubblegum")],
        BubblegumProgramId
      );

      // Attempt to initialize the nonce account. Since localnet is not torn down between suites,
      // there is some shared state. Specifically, the Bubblegum suite may initialize this account
      // if it is run first. Thus even in the case of an error, we proceed.
      try {
        await Bubblegum.rpc.initializeNonce({
          accounts: {
            nonce: noncePDAKey,
            payer: payer.publicKey,
            systemProgram: SystemProgram.programId,
          },
          signers: [payer],
        });
      } catch(e) {
        console.log("Bubblegum nonce PDA already initialized by other suite")
      }
    });

    describe("native sol projects", async () => {
      let creatorPaymentWallet: Keypair;
      beforeEach(async () => {
        creatorAddress = Keypair.generate();
        creatorPaymentWallet = Keypair.generate();
        nftBuyer = Keypair.generate();
        gumballMachineAcctKeypair = Keypair.generate();
        merkleRollKeypair = Keypair.generate();

        baseGumballMachineInitProps = {
          maxDepth: 3,
          maxBufferSize: 8,
          urlBase: strToByteArray("https://arweave.net/Rmg4pcIv-0FQ7M7X838p2r592Q4NU63Fj7o7XsvBHEEl"),
          nameBase: strToByteArray("zfgfsxrwieciemyavrpkuqehkmhqmnim"),
          symbol: strToByteArray("12345678"), 
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
          maxMintSize: new BN(10),
          maxItems: new BN(250),
        };
  
        // Give creator enough funds to produce accounts for NFT
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(creatorAddress.publicKey, LAMPORTS_PER_SOL),
          "confirmed"
        );
  
        await initializeGumballMachine(creatorAddress, gumballMachineAcctKeypair, GUMBALL_MACHINE_ACCT_SIZE, merkleRollKeypair, MERKLE_ROLL_ACCT_SIZE, baseGumballMachineInitProps, NATIVE_MINT);
        await addConfigLines(creatorAddress, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE, strToByteUint8Array("uluvnpwncgchwnbqfpbtdlcpdthc"), Buffer.from("uluvnpwncgchwnbqfpbtdlcpdthc"));
      });
      describe("dispense nft sol instruction", async () => {
        beforeEach(async () => {
          // Give the recipient address enough money to not get rent exempt
          await GumballMachine.provider.connection.confirmTransaction(
            await GumballMachine.provider.connection.requestAirdrop(baseGumballMachineInitProps.receiver, LAMPORTS_PER_SOL),
            "confirmed"
          );
  
          // Fund the NFT Buyer
          await GumballMachine.provider.connection.confirmTransaction(
            await GumballMachine.provider.connection.requestAirdrop(nftBuyer.publicKey, LAMPORTS_PER_SOL),
            "confirmed"
          );
        });
        describe("transaction atomicity attacks fail", async () => {
          let dispenseNFTForSolInstr;
          let dummyNewAcctKeypair;
          let dummyInstr;

          beforeEach(async () => {
            dispenseNFTForSolInstr = await createDispenseNFTForSolIx(new BN(1), nftBuyer, baseGumballMachineInitProps.receiver, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey, GummyrollProgramId, BubblegumProgramId, GumballMachine);
            dummyNewAcctKeypair = Keypair.generate();
            dummyInstr = SystemProgram.createAccount({
              fromPubkey: payer.publicKey,
              newAccountPubkey: dummyNewAcctKeypair.publicKey,
              lamports: 10000000,
              space: 100,
              programId: GumballMachine.programId,
            });
          });
          it("Cannot dispense NFT for SOL with subsequent instructions in transaction", async () => {
            const tx = new Transaction().add(dispenseNFTForSolInstr).add(dummyInstr);
            try {
              await GumballMachine.provider.send(tx, [nftBuyer, payer, dummyNewAcctKeypair], {
                commitment: "confirmed",
              })
              assert(false, "Dispense should fail when part of transaction with multiple instructions, but it succeeded");
            } catch(e) {}
          });
          it("Cannot dispense NFT for SOL with prior instructions in transaction", async () => {
            const tx = new Transaction().add(dummyInstr).add(dispenseNFTForSolInstr);
            try {
              await GumballMachine.provider.send(tx, [nftBuyer, payer, dummyNewAcctKeypair], {
                commitment: "confirmed",
              })
              assert(false, "Dispense should fail when part of transaction with multiple instructions, but it succeeded");
            } catch(e) {}
          });
        });
        it("Can dispense single NFT paid in sol", async () => {
          // Give the recipient address enough money to not get rent exempt
          await GumballMachine.provider.connection.confirmTransaction(
            await GumballMachine.provider.connection.requestAirdrop(baseGumballMachineInitProps.receiver, LAMPORTS_PER_SOL),
            "confirmed"
          );
  
          // Fund the NFT Buyer
          await GumballMachine.provider.connection.confirmTransaction(
            await GumballMachine.provider.connection.requestAirdrop(nftBuyer.publicKey, LAMPORTS_PER_SOL),
            "confirmed"
          );
    
          const nftBuyerBalanceBeforePurchase = await connection.getBalance(nftBuyer.publicKey);
          const creatorBalanceBeforePurchase = await connection.getBalance(baseGumballMachineInitProps.receiver);
  
          // Purchase the compressed NFT with SOL
          await dispenseCompressedNFTForSol(new BN(1), nftBuyer, baseGumballMachineInitProps.receiver, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey);
          const nftBuyerBalanceAfterPurchase = await connection.getBalance(nftBuyer.publicKey);
          const creatorBalanceAfterPurchase = await connection.getBalance(baseGumballMachineInitProps.receiver);
    
          // Assert on how the creator and buyer's balances changed
          assert(
            await creatorBalanceAfterPurchase === (creatorBalanceBeforePurchase + val(baseGumballMachineInitProps.price).toNumber()),
            "Creator balance did not update as expected after NFT purchase"
          );
    
          assert(
            await nftBuyerBalanceAfterPurchase === (nftBuyerBalanceBeforePurchase - val(baseGumballMachineInitProps.price).toNumber()),
            "NFT purchaser balance did not decrease as expected after NFT purchase"
          );
        });
      });
      // @notice: We only test admin instructions on SOL projects because they are completely (for now) independent of project mint
      describe("admin instructions", async () => {
        it("Can update config lines", async () => {
          await updateConfigLines(creatorAddress, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE, Buffer.from("aaavnpwncgchwnbqfpbtdlcpdaaa"), Buffer.from("aaavnpwncgchwnbqfpbtdlcpdaaa"), new BN(0));
        });
        it("Can update gumball header", async () => {
          const newGumballMachineHeader: UpdateHeaderMetadataInstructionArgs = {
            urlBase: strToByteArray("https://arweave.net/bzdjillretjcraaxawlnhqrhmexzbsixyajrlzhfcvcc"),
            nameBase: strToByteArray("wmqeslreeondhmcmtfebrwqnqcoasbye"),
            symbol: strToByteArray("abcdefgh"), 
            sellerFeeBasisPoints: 50,
            isMutable: false,
            retainAuthority: false,
            price: new BN(100),
            goLiveDate: new BN(5678.0),
            botWallet: Keypair.generate().publicKey,
            authority: Keypair.generate().publicKey,
            maxMintSize: new BN(15)
          };

          const expectedOnChainHeader: GumballMachineHeader = {
            urlBase: newGumballMachineHeader.urlBase,
            nameBase: newGumballMachineHeader.nameBase,
            symbol: newGumballMachineHeader.symbol, 
            sellerFeeBasisPoints: newGumballMachineHeader.sellerFeeBasisPoints,
            isMutable: newGumballMachineHeader.isMutable ? 1 : 0,
            retainAuthority: newGumballMachineHeader.retainAuthority ? 1 : 0,
            padding: [0,0,0,0],
            price: newGumballMachineHeader.price,
            goLiveDate: newGumballMachineHeader.goLiveDate,
            mint: NATIVE_MINT,
            botWallet: newGumballMachineHeader.botWallet,
            receiver: baseGumballMachineInitProps.receiver,
            authority: newGumballMachineHeader.authority,
            collectionKey: baseGumballMachineInitProps.collectionKey,
            creatorAddress: creatorAddress.publicKey,
            extensionLen: baseGumballMachineInitProps.extensionLen,
            maxMintSize: newGumballMachineHeader.maxMintSize,
            remaining: new BN(0),
            maxItems: baseGumballMachineInitProps.maxItems,
            totalItemsAdded: new BN(0)
          }
          await updateHeaderMetadata(creatorAddress, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, newGumballMachineHeader, expectedOnChainHeader);
        });
        it("Can destroy gumball machine and reclaim lamports", async () => {
          await destroyGumballMachine(gumballMachineAcctKeypair, creatorAddress);
        });
      });
    });
    describe("spl token projects", async () => {
      let someMint: PublicKey;
      let creatorReceiverTokenAccount;
      let botWallet;
      let nftBuyerTokenAccount;

      beforeEach(async () => {
        creatorAddress = Keypair.generate();
        gumballMachineAcctKeypair = Keypair.generate();
        merkleRollKeypair = Keypair.generate();
        nftBuyer = Keypair.generate();
        botWallet = Keypair.generate();

        // Give creator enough funds to produce accounts for gumball-machine
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(creatorAddress.publicKey, 4 * LAMPORTS_PER_SOL),
          "confirmed"
        );

        someMint = await createMint(connection, payer, payer.publicKey, null, 9);
        creatorReceiverTokenAccount = await getOrCreateAssociatedTokenAccount(connection, payer, someMint, creatorAddress.publicKey);

        baseGumballMachineInitProps = {
          maxDepth: 3,
          maxBufferSize: 8,
          urlBase: strToByteArray("https://arweave.net/Rmg4pcIv-0FQ7M7X838p2r592Q4NU63Fj7o7XsvBHEEl"),
          nameBase: strToByteArray("zfgfsxrwieciemyavrpkuqehkmhqmnim"),
          symbol: strToByteArray("12345678"), 
          sellerFeeBasisPoints: 100,
          isMutable: true,
          retainAuthority: true,
          price: new BN(10),
          goLiveDate: new BN(1234.0),
          botWallet: botWallet.publicKey,
          receiver: creatorReceiverTokenAccount.address,
          authority: creatorAddress.publicKey,
          collectionKey: SystemProgram.programId, // 0x0 -> no collection key
          extensionLen: new BN(28),
          maxMintSize: new BN(10),
          maxItems: new BN(250),
        };
  
        await initializeGumballMachine(creatorAddress, gumballMachineAcctKeypair, GUMBALL_MACHINE_ACCT_SIZE, merkleRollKeypair, MERKLE_ROLL_ACCT_SIZE, baseGumballMachineInitProps, someMint);
        await addConfigLines(creatorAddress, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE, Buffer.from("uluvnpwncgchwnbqfpbtdlcpdthc" + "aauvnpwncgchwnbqfpbtdlcpdthc"), Buffer.from("uluvnpwncgchwnbqfpbtdlcpdthc" + "aauvnpwncgchwnbqfpbtdlcpdthc"));

        // Create and fund the NFT pruchaser
        nftBuyerTokenAccount = await getOrCreateAssociatedTokenAccount(connection, payer, someMint, nftBuyer.publicKey);
        await mintTo(connection, payer, someMint, nftBuyerTokenAccount.address, payer, 50);
      });
      describe("transaction atomicity attacks fail", async () => {
        let dispenseNFTForTokensInstr;
        let dummyNewAcctKeypair;
        let dummyInstr;

        beforeEach(async () => {
          dispenseNFTForTokensInstr = await createDispenseNFTForTokensIx(new BN(1), nftBuyer, nftBuyerTokenAccount.address, creatorReceiverTokenAccount.address, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey, GummyrollProgramId, BubblegumProgramId, GumballMachine);
          dummyNewAcctKeypair = Keypair.generate();
          dummyInstr = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: dummyNewAcctKeypair.publicKey,
            lamports: 10000000,
            space: 100,
            programId: GumballMachine.programId,
          });
        });
        it("Cannot dispense NFT for tokens with subsequent instructions in transaction", async () => {
          const tx = new Transaction().add(dispenseNFTForTokensInstr).add(dummyInstr);
          try {
            await GumballMachine.provider.send(tx, [nftBuyer, payer, dummyNewAcctKeypair], {
              commitment: "confirmed",
            })
            assert(false, "Dispense should fail when part of transaction with multiple instructions, but it succeeded");
          } catch(e) {}
        });
        it("Cannot dispense NFT for SOL with prior instructions in transaction", async () => {
          const tx = new Transaction().add(dummyInstr).add(dispenseNFTForTokensInstr);
          try {
            await GumballMachine.provider.send(tx, [nftBuyer, payer, dummyNewAcctKeypair], {
              commitment: "confirmed",
            })
            assert(false, "Dispense should fail when part of transaction with multiple instructions, but it succeeded");
          } catch(e) {}
        });
      });
      it("Can dispense multiple NFTs paid in token", async () => {
        let buyerTokenAccount = await getAccount(connection, nftBuyerTokenAccount.address);
        await dispenseCompressedNFTForTokens(new BN(1), nftBuyer, nftBuyerTokenAccount.address, creatorReceiverTokenAccount.address, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey);
  
        let newCreatorTokenAccount = await getAccount(connection, creatorReceiverTokenAccount.address);
        let newBuyerTokenAccount = await getAccount(connection, nftBuyerTokenAccount.address);
        
        assert(
          Number(newCreatorTokenAccount.amount) === Number(creatorReceiverTokenAccount.amount) + val(baseGumballMachineInitProps.price).toNumber(),
          "The creator did not receive their payment as expected"
        );
  
        assert(
          Number(newBuyerTokenAccount.amount) === Number(buyerTokenAccount.amount) - val(baseGumballMachineInitProps.price).toNumber(),
          "The nft buyer did not pay for the nft as expected"
        );

        await dispenseCompressedNFTForTokens(new BN(1), nftBuyer, nftBuyerTokenAccount.address, creatorReceiverTokenAccount.address, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey);

        creatorReceiverTokenAccount = newCreatorTokenAccount;
        buyerTokenAccount = newBuyerTokenAccount;
        newCreatorTokenAccount = await getAccount(connection, creatorReceiverTokenAccount.address);
        newBuyerTokenAccount = await getAccount(connection, nftBuyerTokenAccount.address);
        
        assert(
          Number(newCreatorTokenAccount.amount) === Number(creatorReceiverTokenAccount.amount) + val(baseGumballMachineInitProps.price).toNumber(),
          "The creator did not receive their payment as expected"
        );
  
        assert(
          Number(newBuyerTokenAccount.amount) === Number(buyerTokenAccount.amount) - val(baseGumballMachineInitProps.price).toNumber(),
          "The nft buyer did not pay for the nft as expected"
        );
      });
    });
  });
});
