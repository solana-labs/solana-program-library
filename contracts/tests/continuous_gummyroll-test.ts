import * as anchor from "@project-serum/anchor";
import { Gummyroll } from "../target/types/gummyroll";
import { Program, Provider, } from "@project-serum/anchor";
import {
  Connection as web3Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { assert } from "chai";

import { buildTree, getProofOfLeaf, updateTree, Tree, getProofOfAssetFromServer, checkProof } from "./merkle-tree";
import { decodeMerkleRoll, getMerkleRollAccountSize } from "./merkle-roll-serde";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";

const HOST = "127.0.0.1";
const TREE_RPC_HOST = HOST;
const CONNECTION_URL = `http://${HOST}:8899`;
const TREE_RPC_PORT = "9090";
const PROOF_URL = `http://${TREE_RPC_HOST}:${TREE_RPC_PORT}`;

console.log(`Using RPC: ${CONNECTION_URL}`);

// @ts-ignore
let Gummyroll;

function chunk<T>(arr: T[], size: number): T[][] {
  return Array.from({ length: Math.ceil(arr.length / size) }, (_: any, i: number) =>
    arr.slice(i * size, i * size + size)
  );
}

describe("gummyroll-continuous", () => {
  let connection: web3Connection;
  let wallet: NodeWallet;
  let offChainTree: ReturnType<typeof buildTree>;
  let merkleRollKeypair: Keypair;
  let payer: Keypair;

  const MAX_SIZE = 1024;
  const MAX_DEPTH = 20;
  const NUM_TO_SEND = 50;
  // This is hardware dependent... if too large, then majority of tx's will fail to confirm
  const BATCH_SIZE = 25;

  async function createEmptyTreeOnChain(
    payer: Keypair
  ): Promise<Keypair> {
    const merkleRollKeypair = Keypair.generate();
    const requiredSpace = getMerkleRollAccountSize(MAX_DEPTH, MAX_SIZE);
    const allocAccountIx = SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: merkleRollKeypair.publicKey,
      lamports:
        await Gummyroll.provider.connection.getMinimumBalanceForRentExemption(
          requiredSpace
        ),
      space: requiredSpace,
      programId: Gummyroll.programId,
    });

    const initGummyrollIx = Gummyroll.instruction.initEmptyGummyroll(
      MAX_DEPTH,
      MAX_SIZE,
      {
        accounts: {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
          appendAuthority: payer.publicKey,
        },
        signers: [payer],
      }
    );

    const tx = new Transaction().add(allocAccountIx).add(initGummyrollIx);
    let txid = await Gummyroll.provider.send(tx, [payer, merkleRollKeypair], {
      commitment: "confirmed",
      skipPreflight: true,
    });
    console.log(txid);
    return merkleRollKeypair
  }

  function createEmptyTreeOffChain(): Tree {
    const leaves = Array(2 ** MAX_DEPTH).fill(Buffer.alloc(32));
    let tree = buildTree(leaves);
    return tree;
  }

  beforeEach(async () => {
    payer = Keypair.generate();
    connection = new web3Connection(
      CONNECTION_URL,
      {
        commitment: 'confirmed'
      }
    );
    wallet = new NodeWallet(payer)
    anchor.setProvider(new Provider(connection, wallet, { commitment: connection.commitment, skipPreflight: true }));
    Gummyroll = anchor.workspace.Gummyroll as Program<Gummyroll>;
    console.log(Gummyroll.programId.toString());
    await Gummyroll.provider.connection.confirmTransaction(
      await Gummyroll.provider.connection.requestAirdrop(payer.publicKey, 1e10),
      "confirmed"
    );

    merkleRollKeypair = await createEmptyTreeOnChain(payer);

    console.log("TREE ID: ", merkleRollKeypair.publicKey.toString())

    const merkleRoll = await Gummyroll.provider.connection.getAccountInfo(
      merkleRollKeypair.publicKey
    );

    let onChainMerkle = decodeMerkleRoll(merkleRoll.data);

    // Check header bytes are set correctly
    assert(onChainMerkle.header.maxDepth === MAX_DEPTH, `Max depth does not match ${onChainMerkle.header.maxDepth}, expected ${MAX_DEPTH}`);
    assert(onChainMerkle.header.maxBufferSize === MAX_SIZE, `Max buffer size does not match ${onChainMerkle.header.maxBufferSize}, expected ${MAX_SIZE}`);

    assert(
      onChainMerkle.header.authority.equals(payer.publicKey),
      "Failed to write auth pubkey"
    );

    offChainTree = createEmptyTreeOffChain();

    assert(
      onChainMerkle.roll.changeLogs[0].root.equals(new PublicKey(offChainTree.root)),
      "On chain root does not match root passed in instruction"
    );
  });

  // Will be used in future test
  async function createReplaceIx(merkleRollKeypair: Keypair, payer: Keypair, leafIdx: number, maxDepth: number) {
    /// Empty nodes are special, so we have to create non-zero leaf for index 0
    let newLeaf = Buffer.alloc(32, Buffer.from(Uint8Array.from([1 + leafIdx])));

    const assetProof = await getProofOfAssetFromServer(PROOF_URL, merkleRollKeypair.publicKey, leafIdx);
    checkProof(leafIdx, assetProof.root, assetProof.hash, assetProof.proof);

    const nodeProof = assetProof.proof.map((node) => { return { pubkey: new PublicKey(node), isSigner: false, isWritable: false } });
    const replaceLeafIx = Gummyroll.instruction.replaceLeaf(
      { inner: Array.from(new PublicKey(assetProof.root).toBuffer()) },
      { inner: Array.from(new PublicKey(assetProof.hash).toBuffer()) },
      { inner: Array.from(newLeaf) },
      leafIdx,
      {
        accounts: {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
        },
        signers: [payer],
        remainingAccounts: nodeProof,
      }
    );
    return replaceLeafIx;
  }

  it(`${MAX_SIZE} leaves replaced in batches of ${BATCH_SIZE}`, async () => {
    let indicesToSend = [];
    for (let i = 0; i < NUM_TO_SEND; i++) {
      indicesToSend.push(i);
    };
    const indicesToSync = indicesToSend;

    let numCompleted = 0;
    while (indicesToSend.length > 0) {
      let batchesToSend = chunk<number>(indicesToSend, BATCH_SIZE);
      let indicesLeft: number[] = [];
      for (const [_j, batch] of batchesToSend.entries()) {
        const txIds = [];
        const txIdToIndex: Record<string, number> = {};
        for (const i of batch) {
          const tx = new Transaction().add(await createReplaceIx(merkleRollKeypair, payer, i, MAX_DEPTH));

          tx.feePayer = payer.publicKey;
          tx.recentBlockhash = (await connection.getLatestBlockhash('singleGossip')).blockhash;

          await wallet.signTransaction(tx);
          const rawTx = tx.serialize();

          txIds.push(
            connection.sendRawTransaction(rawTx, { skipPreflight: true })
              .then((txId) => {
                txIdToIndex[txId] = i;
                return txId
              })
              .catch((reason) => {
                console.error(reason);
                return i
              })
          );
        }
        const sendResults: (string | number)[] = (await Promise.all(txIds));
        const batchToConfirm = sendResults.filter((result) => typeof result === "string") as string[];
        const txsToReplay = sendResults.filter((err) => typeof err === "number") as number[];
        if (txsToReplay.length) {
          indicesLeft = indicesLeft.concat(txsToReplay as number[]);
          console.log(`${txsToReplay.length} tx's failed in batch`)
        }

        await Promise.all(batchToConfirm.map(async (txId) => {
          const confirmation = await connection.confirmTransaction(txId, "confirmed")
          if (confirmation.value.err && txIdToIndex[txId]) {
            console.log(txIdToIndex[txId], "failed", confirmation.value.err);
            txsToReplay.push(txIdToIndex[txId]);
            throw new Error(`Failed to process transaction: ${txId}`);
          }
          return confirmation;
        }));

        numCompleted += batchToConfirm.length - txsToReplay.length;
        console.log("Successfully completed: ", numCompleted);

        indicesLeft = indicesLeft.concat(txsToReplay);
      }

      indicesToSend = indicesLeft;
    }

    // Create expected off-chain tree state
    for (const i of indicesToSync) {
      updateTree(offChainTree, Buffer.alloc(32, Buffer.from(Uint8Array.from([1 + i]))), i);
    }

    const merkleRoll = await Gummyroll.provider.connection.getAccountInfo(
      merkleRollKeypair.publicKey
    );
    let onChainMerkle = decodeMerkleRoll(merkleRoll.data);

    const onChainRoot = onChainMerkle.roll.changeLogs[onChainMerkle.roll.activeIndex].root;
    const treeRoot = new PublicKey(offChainTree.root);
    assert(
      onChainRoot.equals(treeRoot),
      "On chain root does not match root passed in instruction"
    );
  });
});
