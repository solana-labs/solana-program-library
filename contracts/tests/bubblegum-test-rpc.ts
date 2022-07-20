import * as anchor from "@project-serum/anchor";
import { keccak_256 } from "js-sha3";
import { BN, Provider, Program } from "@project-serum/anchor";
import { Bubblegum } from "../target/types/bubblegum";
import { Gummyroll } from "../target/types/gummyroll";
import fetch from "node-fetch";
import { PROGRAM_ID } from "@metaplex-foundation/mpl-token-metadata";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  SYSVAR_RENT_PUBKEY, AccountMeta,
} from "@solana/web3.js";
import { assert } from "chai";
import {
  createMintV1Instruction,
  createDecompressV1Instruction,
  createTransferInstruction,
  createDelegateInstruction,
  createRedeemInstruction,
  createCancelRedeemInstruction,
  createCreateTreeInstruction
} from "../sdk/bubblegum/src/generated";

import { buildTree, checkProof, Tree } from "./merkle-tree";
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
import { TokenProgramVersion, Version } from "../sdk/bubblegum/src/generated";
import { sleep } from "@metaplex-foundation/amman/dist/utils";
import { verbose } from "sqlite3";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { CANDY_WRAPPER_PROGRAM_ID, execute, logTx, num16ToBuffer, bufferToArray } from "../sdk/utils";
// TODO: cleanup this test file using the convenience methods and remove all .send calls
import { computeDataHash, computeCreatorHash } from "../sdk/bubblegum/src/convenience";

// @ts-ignore
let Bubblegum;
// @ts-ignore
let GummyrollProgramId;

interface TreeProof {
  root: string,
  proof: AccountMeta[]
}

async function getProof(asset: PublicKey): Promise<TreeProof> {
  let resp = await fetch("http://localhost:9090", {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
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

describe("bubblegum", () => {
  // Configure the client to use the local cluster.
  let offChainTree: Tree;
  let treeAuthority: PublicKey;
  let merkleRollKeypair: Keypair;

  const MAX_SIZE = 64;
  const MAX_DEPTH = 20;

  let payer = Keypair.generate();
  let destination = Keypair.generate();
  let delegateKey = Keypair.generate();
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
  Bubblegum = anchor.workspace.Bubblegum as Program<Bubblegum>;
  GummyrollProgramId = anchor.workspace.Gummyroll.programId;

  async function createTreeOnChain(
    payer: Keypair,
    destination: Keypair,
    delegate: Keypair
  ): Promise<[Keypair, Tree, PublicKey]> {
    const merkleRollKeypair = Keypair.generate();

    await Bubblegum.provider.connection.confirmTransaction(
      await Bubblegum.provider.connection.requestAirdrop(payer.publicKey, 10e9),
      "confirmed"
    );
    await Bubblegum.provider.connection.confirmTransaction(
      await Bubblegum.provider.connection.requestAirdrop(
        destination.publicKey,
        10e9
      ),
      "confirmed"
    );
    await Bubblegum.provider.connection.confirmTransaction(
      await Bubblegum.provider.connection.requestAirdrop(
        delegate.publicKey,
        10e9
      ),
      "confirmed"
    );
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

  describe("Testing bubblegum", () => {
    beforeEach(async () => {
      let [computedMerkleRoll, computedOffChainTree, computedTreeAuthority] =
        await createTreeOnChain(payer, destination, delegateKey);
      merkleRollKeypair = computedMerkleRoll;
      offChainTree = computedOffChainTree;
      treeAuthority = computedTreeAuthority;
    });
    it("Mint to tree", async () => {
      function rngf(stop, i) {
        if (i > stop) {
          return (stop - i) % stop;
        }
        return i
      }

      for (let i = 0; i < 10000; i++) {
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
          { message: metadata }
        );
        console.log(" - Minting to tree");
        const mintTx = await Bubblegum.provider.send(
          new Transaction().add(mintIx),
          [payer],
          {
            skipPreflight: true,
            commitment: "confirmed",
          }
        );
        const dataHash = computeDataHash(metadata.sellerFeeBasisPoints, mintIx);
        const creatorHash = computeCreatorHash(metadata.creators);

        console.log(" - Transferring Ownership");
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
        {
          let { root, proof } = await getProof(asset);
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
              index: i,
            }
          );
          transferIx.keys[1].isSigner = true;
          transferIx.keys = [...transferIx.keys, ...proof];

          await execute(Bubblegum.provider, [transferIx], [payer]);
        }

        console.log(" - Delegating Ownership");
        {
          let { root, proof } = await getProof(asset);
          let delegateIx = await createDelegateInstruction(
            {
              authority: treeAuthority,
              candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
              owner: destination.publicKey,
              previousDelegate: destination.publicKey,
              newDelegate: delegateKey.publicKey,
              gummyrollProgram: GummyrollProgramId,
              merkleSlab: merkleRollKeypair.publicKey,
            },
            {
              root: bufferToArray(bs58.decode(root)),
              dataHash,
              creatorHash,
              nonce: leafNonce,
              index: i,
            }
          );
          delegateIx.keys = [...delegateIx.keys, ...proof];
          await execute(Bubblegum.provider, [delegateIx], [destination]);
        }

        console.log(" - Transferring Ownership (through delegate)");
        {
          let { root, proof } = await getProof(asset);
          let delTransferIx = createTransferInstruction(
            {
              authority: treeAuthority,
              candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
              owner: destination.publicKey,
              delegate: delegateKey.publicKey,
              newOwner: payer.publicKey,
              gummyrollProgram: GummyrollProgramId,
              merkleSlab: merkleRollKeypair.publicKey,
            },
            {
              root: bufferToArray(bs58.decode(root)),
              dataHash,
              creatorHash,
              nonce: leafNonce,
              index: i,
            }
          );
          delTransferIx.keys[2].isSigner = true;
          delTransferIx.keys = [...delTransferIx.keys, ...proof];
          let delTransferTx = await Bubblegum.provider.send(
            new Transaction().add(delTransferIx),
            [delegateKey],
            {
              skipPreflight: true,
              commitment: "confirmed",
            }
          );
        }

        let [voucher] = await PublicKey.findProgramAddress(
          [
            Buffer.from("voucher", "utf8"),
            merkleRollKeypair.publicKey.toBuffer(),
            leafNonce.toBuffer("le", 8)
          ],
          Bubblegum.programId
        );
        if (i % 2 == 0) {
          console.log(" - Redeeming Leaf", voucher.toBase58());
          {
            let { root, proof } = await getProof(asset);
            let redeemIx = createRedeemInstruction(
              {
                authority: treeAuthority,
                candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
                owner: payer.publicKey,
                delegate: payer.publicKey,
                gummyrollProgram: GummyrollProgramId,
                merkleSlab: merkleRollKeypair.publicKey,
                voucher: voucher,
              },
              {
                root: bufferToArray(bs58.decode(root)),
                dataHash,
                creatorHash,
                nonce: leafNonce,
                index: i,
              }
            );
            redeemIx.keys = [...redeemIx.keys, ...proof];
            let redeemTx = await Bubblegum.provider.send(
              new Transaction().add(redeemIx),
              [payer],
              {
                skipPreflight: true,
                commitment: "confirmed",
              }
            );
          }
          console.log(" - Cancelling redeem (reinserting to tree)");
          {
            let merkleRollAccount =
              await Bubblegum.provider.connection.getAccountInfo(
                merkleRollKeypair.publicKey
              );
            let merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
            let { root, proof } = await getProof(asset);
            console.log("rpc root ", root);


            console.log("on chain roots ")
            merkleRoll.roll.changeLogs.map(cl => console.log(cl.root.toBase58()))

            const cancelRedeemIx = createCancelRedeemInstruction(
              {
                authority: treeAuthority,
                candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
                owner: payer.publicKey,
                gummyrollProgram: GummyrollProgramId,
                merkleSlab: merkleRollKeypair.publicKey,
                voucher: voucher,
              },
              {
                root: bufferToArray(bs58.decode(root)),
              }
            );
            cancelRedeemIx.keys = [...cancelRedeemIx.keys, ...proof];
            let cancelRedeemTx = await Bubblegum.provider.send(
              new Transaction().add(cancelRedeemIx),
              [payer],
              {
                commitment: "confirmed",
              }
            );
          }

          console.log(" - Decompressing leaf");
          {
            let { root, proof } = await getProof(asset);
            let redeemIx = createRedeemInstruction(
              {
                authority: treeAuthority,
                owner: payer.publicKey,
                candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
                delegate: payer.publicKey,
                gummyrollProgram: GummyrollProgramId,
                merkleSlab: merkleRollKeypair.publicKey,
                voucher: voucher,
              },
              {
                root: bufferToArray(bs58.decode(root)),
                dataHash,
                creatorHash,
                nonce: leafNonce,
                index: i,
              }
            );
            redeemIx.keys = [...redeemIx.keys, ...proof];
            let redeemTX = await Bubblegum.provider.send(
              new Transaction().add(redeemIx),
              [payer],
              {
                commitment: "confirmed",
              }
            );
          }

          console.log("Decompressing - ", asset.toBase58())

          let [mintAuthority] = await PublicKey.findProgramAddress(
            [asset.toBuffer()],
            Bubblegum.programId
          );

          const getMetadata = async (
            mint: anchor.web3.PublicKey
          ): Promise<anchor.web3.PublicKey> => {
            return (
              await anchor.web3.PublicKey.findProgramAddress(
                [Buffer.from("metadata"), PROGRAM_ID.toBuffer(), mint.toBuffer()],
                PROGRAM_ID
              )
            )[0];
          };

          const getMasterEdition = async (
            mint: anchor.web3.PublicKey
          ): Promise<anchor.web3.PublicKey> => {
            return (
              await anchor.web3.PublicKey.findProgramAddress(
                [
                  Buffer.from("metadata"),
                  PROGRAM_ID.toBuffer(),
                  mint.toBuffer(),
                  Buffer.from("edition"),
                ],
                PROGRAM_ID
              )
            )[0];
          };
          let decompressIx = createDecompressV1Instruction(
            {
              voucher: voucher,
              owner: payer.publicKey,
              tokenAccount: await Token.getAssociatedTokenAddress(
                ASSOCIATED_TOKEN_PROGRAM_ID,
                TOKEN_PROGRAM_ID,
                asset,
                payer.publicKey
              ),
              mint: asset,
              mintAuthority: mintAuthority,
              metadata: await getMetadata(asset),
              masterEdition: await getMasterEdition(asset),
              sysvarRent: SYSVAR_RENT_PUBKEY,
              tokenMetadataProgram: PROGRAM_ID,
              associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            },
            {
              metadata,
            }
          );

          let decompressTx = await Bubblegum.provider.send(
            new Transaction().add(decompressIx),
            [payer],
            {
              commitment: "confirmed",
            }
          );
        }
      }
    });
  });
});
