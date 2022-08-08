import * as anchor from "@project-serum/anchor";
import {keccak_256} from "js-sha3";
import {BN, Provider, Program} from "@project-serum/anchor";
import {Bubblegum} from "../target/types/bubblegum";
import {Gummyroll} from "../target/types/gummyroll";
import fetch from "node-fetch";
import {PROGRAM_ID} from "@metaplex-foundation/mpl-token-metadata";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  SYSVAR_RENT_PUBKEY, AccountMeta,
} from "@solana/web3.js";
import {assert} from "chai";
import {
  createMintV1Instruction,
  createDecompressV1Instruction,
  createTransferInstruction,
  createDelegateInstruction,
  createRedeemInstruction,
  createCancelRedeemInstruction,
  createCreateTreeInstruction
} from "../sdk/bubblegum/src/generated";

import {buildTree, checkProof, Tree} from "./merkle-tree";
import {
  decodeMerkleRoll,
  getMerkleRollAccountSize,
  assertOnChainMerkleRollProperties,
  createTransferAuthorityIx,
} from "../sdk/gummyroll";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  Token,
} from "@solana/spl-token";
import {CANDY_WRAPPER_PROGRAM_ID, execute, logTx} from "../sdk/utils";
import {TokenProgramVersion, Version} from "../sdk/bubblegum/src/generated";
import {sleep} from "@metaplex-foundation/amman/dist/utils";
import {bs58} from "@project-serum/anchor/dist/cjs/utils/bytes";
import  retry from "retry-as-promised";
import {as} from "pg-promise";
// @ts-ignore
let Bubblegum;
// @ts-ignore
let GummyrollProgramId;

/// Converts to Uint8Array
function bufferToArray(buffer: Buffer): number[] {
  const nums = [];
  for (let i = 0; i < buffer.length; i++) {
    nums.push(buffer.at(i));
  }
  return nums;
}

interface TreeProof {
  root: string,
  proof: AccountMeta[]
}

async function getAsset(asset: PublicKey): Promise<any> {
  let resp = await fetch("https://rpc.aws.metaplex.com", {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      jsonrpc: "2.0", id: "stupid", method: "get_asset", params: [asset.toBase58()]
    })
  });
  let js = await resp.json();

  return js;
}

async function getProof(asset: PublicKey): Promise<TreeProof> {
  let resp = await fetch("https://rpc.aws.metaplex.com", {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      jsonrpc: "2.0", id: "stupid", method: "get_asset_proof", params: [asset.toBase58()]
    })
  });
  let js = await resp.json();
  const proofNodes: Array<AccountMeta> = js.result.proof.map((key) => {
    return {
      pubkey: new PublicKey(key),
      isWritable: false,
      isSigner: false,
    };
  });
  return {
    root: js.result.root,
    proof: proofNodes
  };
}

// Configure the client to use the local cluster.
let offChainTree: Tree;
let treeAuthority: PublicKey;
let merkleRollKeypair: Keypair;

const MAX_SIZE = 64;
const MAX_DEPTH = 20;
const p_seed = new TextEncoder().encode(process.env["SEED"] + "p")
const d_seed = new TextEncoder().encode(process.env["SEED"] + "d")
let payer = Keypair.fromSeed(p_seed);
let destination = Keypair.fromSeed(d_seed);
console.log(payer.publicKey.toBase58())
console.log(destination.publicKey.toBase58())
let connection = new web3Connection("https://liquid.testnet.rpcpool.com/5ebea512d12be102f53d319dafc8", {
  commitment: "confirmed",
});
let wallet = new NodeWallet(payer);
anchor.setProvider(
  new Provider(connection, wallet, {
    commitment: connection.commitment,
    skipPreflight: true,
  })
);
Bubblegum = anchor.workspace.Bubblegum as Program<Bubblegum>;
GummyrollProgramId = anchor.workspace.Gummyroll.programId;

async function createTreeOnChain(
  payer: Keypair,
): Promise<[Keypair, Tree, PublicKey]> {
  const merkleRollKeypair = Keypair.generate();
  const requiredSpace = getMerkleRollAccountSize(MAX_DEPTH, MAX_SIZE);
  const leaves = Array(2 ** MAX_DEPTH).fill(Buffer.alloc(32));
  const tree = buildTree(leaves);

  const allocAccountIx = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: merkleRollKeypair.publicKey,
    lamports:
      await Bubblegum.provider.connection.getMinimumBalanceForRentExemption(
        requiredSpace
      ),
    space: requiredSpace,
    programId: GummyrollProgramId,
  });

  let [authority] = await PublicKey.findProgramAddress(
    [merkleRollKeypair.publicKey.toBuffer()],
    Bubblegum.programId
  );

  const initGummyrollIx = createCreateTreeInstruction(
    {
      treeCreator: payer.publicKey,
      candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
      payer: payer.publicKey,
      authority: authority,
      gummyrollProgram: GummyrollProgramId,
      merkleSlab: merkleRollKeypair.publicKey
    },
    {
      maxDepth: MAX_DEPTH,
      maxBufferSize: MAX_SIZE
    }
  );

  let tx = new Transaction().add(allocAccountIx).add(initGummyrollIx);

  await Bubblegum.provider.send(tx, [payer, merkleRollKeypair], {
    commitment: "confirmed",
    skipPreflight: true,
  });

  await assertOnChainMerkleRollProperties(
    Bubblegum.provider.connection,
    MAX_DEPTH,
    MAX_SIZE,
    authority,
    new PublicKey(tree.root),
    merkleRollKeypair.publicKey
  );

  return [merkleRollKeypair, tree, authority];
}


function rngf(stop, i) {
  if (i > stop) {
    return (stop - i) % stop;
  }
  return i
}

async function transfer(index, treeAuthority, data, payer, destination, merkleRollKeypair, leafNonce, root, proof) {
  const dataHash = bufferToArray(
    Buffer.from(keccak_256.digest(data.slice(8)))
  );
  const creatorHash = bufferToArray(Buffer.from(keccak_256.digest([])));

    console.log("Got new proof")
    let transferIx = createTransferInstruction(
      {
        authority: treeAuthority,
        candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
        owner: payer.publicKey,
        delegate: payer.publicKey,
        newOwner: destination.publicKey,
        gummyrollProgram: GummyrollProgramId,
        merkleSlab: merkleRollKeypair.publicKey,
      },
      {
        root: bufferToArray(bs58.decode(root)),
        dataHash,
        creatorHash,
        nonce: leafNonce,
        index: index,
      }
    );
    transferIx.keys[1].isSigner = true;
    transferIx.keys = [...transferIx.keys, ...proof];
    await execute(Bubblegum.provider, [transferIx], [payer], true);
    console.log(" - Transferred Ownership");
}

async function main() {

  let [computedMerkleRoll, computedOffChainTree, computedTreeAuthority] =
    await createTreeOnChain(payer);
  merkleRollKeypair = computedMerkleRoll;
  offChainTree = computedOffChainTree;
  treeAuthority = computedTreeAuthority;

  for (let i = 0; i < 10; i++) {
    const metadata = {
      name: "OH " + i,
      symbol: "OH" + i,
      uri: `https://onlyhands.s3.amazonaws.com/assets/${rngf(300, i)}.json`,
      sellerFeeBasisPoints: 10000,
      primarySaleHappened: false,
      isMutable: false,
      editionNonce: null,
      tokenStandard: null,
      tokenProgramVersion: TokenProgramVersion.Original,
      collection: null,
      uses: null,
      creators: [],
    };
    const mintIx = createMintV1Instruction(
      {
        mintAuthority: payer.publicKey,
        authority: treeAuthority,
        candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
        gummyrollProgram: GummyrollProgramId,
        owner: payer.publicKey,
        delegate: payer.publicKey,
        merkleSlab: merkleRollKeypair.publicKey,
      },
      {message: metadata}
    );
    console.log(" - Minting to tree");
    await Bubblegum.provider.send(
      new Transaction().add(mintIx),
      [payer],
      {
        skipPreflight: true,
        commitment: "confirmed",
      }
    );
    await sleep(1000);
    for (let j = 0; j < 1000; j++) {
      let tx = async function() {
        try {
          const nonceInfo = await (
            Bubblegum.provider.connection as web3Connection
          ).getAccountInfo(treeAuthority);
          const leafNonce = new BN(nonceInfo.data.slice(8, 16), "le").sub(
            new BN(1)
          );
          let [asset] = await PublicKey.findProgramAddress(
            [Buffer.from("asset", "utf8"), merkleRollKeypair.publicKey.toBuffer(), leafNonce.toBuffer("le", 8)],
            Bubblegum.programId
          );
          let {root, proof} = await getProof(asset);
          let assetObj = await getAsset(asset);
          console.log(assetObj)
          let localPayer, localDest;
          if(assetObj.result.ownership.owner != payer.publicKey.toBase58()) {
            localPayer = destination
            localDest = payer;
          } else {
            localPayer = payer;
            localDest = destination;
          }
          console.log("Attempting Transfer")
          await transfer(i, treeAuthority, mintIx.data, localPayer, localDest, merkleRollKeypair, leafNonce, root, proof)
        } catch (e) {
          console.log("Error", e)
          throw e
        }
      }
      await retry(tx, {max: 5})
    }
  }
}


main()
