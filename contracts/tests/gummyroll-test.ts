import * as anchor from "@project-serum/anchor";
import { BN, Provider, Program } from "@project-serum/anchor";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
} from "@solana/web3.js";
import { assert } from "chai";
import * as crypto from "crypto";

import {
  buildTree,
  hash,
  getProofOfLeaf,
  updateTree,
  Tree,
} from "./merkle-tree";
import {
  Gummyroll,
  createReplaceIx,
  createAppendIx,
  createTransferAuthorityIx,
  decodeMerkleRoll,
  getMerkleRollAccountSize,
  createVerifyLeafIx,
  assertOnChainMerkleRollProperties,
  createAllocTreeIx,
} from "../sdk/gummyroll";
import { execute, logTx } from "./utils";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { CANDY_WRAPPER_PROGRAM_ID } from "../sdk/utils";

// @ts-ignore
let Gummyroll;

describe("gummyroll", () => {
  // Configure the client to use the local cluster.
  let offChainTree: Tree;
  let merkleRollKeypair: Keypair;
  let payer: Keypair;
  let connection;
  let wallet;

  const MAX_SIZE = 64;
  const MAX_DEPTH = 14;

  async function createTreeOnChain(
    payer: Keypair,
    numLeaves: number,
    maxDepth?: number,
    maxSize?: number,
    canopyDepth?: number
  ): Promise<[Keypair, Tree]> {
    if (maxDepth === undefined) {
      maxDepth = MAX_DEPTH;
    }
    if (maxSize === undefined) {
      maxSize = MAX_SIZE;
    }
    const merkleRollKeypair = Keypair.generate();

    const leaves = Array(2 ** maxDepth).fill(Buffer.alloc(32));
    for (let i = 0; i < numLeaves; i++) {
      leaves[i] = crypto.randomBytes(32);
    }
    const tree = buildTree(leaves);

    const allocAccountIx = await createAllocTreeIx(
      Gummyroll.provider.connection,
      maxSize,
      maxDepth,
      canopyDepth,
      payer.publicKey,
      merkleRollKeypair.publicKey,
    );

    let tx = new Transaction().add(allocAccountIx);
    if (numLeaves > 0) {
      const root = Array.from(tree.root.map((x) => x));
      const leaf = Array.from(leaves[numLeaves - 1]);
      const proof = getProofOfLeaf(tree, numLeaves - 1).map((node) => {
        return {
          pubkey: new PublicKey(node.node),
          isSigner: false,
          isWritable: false,
        };
      });

      tx = tx.add(
        Gummyroll.instruction.initGummyrollWithRoot(
          maxDepth,
          maxSize,
          root,
          leaf,
          numLeaves - 1,
          "https://arweave.net/<changelog_db_uri>",
          "https://arweave.net/<metadata_db_id>",
          {
            accounts: {
              merkleRoll: merkleRollKeypair.publicKey,
              authority: payer.publicKey,
              appendAuthority: payer.publicKey,
              candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
            },
            signers: [payer],
            remainingAccounts: proof,
          }
        )
      );
    } else {
      tx = tx.add(
        Gummyroll.instruction.initEmptyGummyroll(maxDepth, maxSize, {
          accounts: {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: payer.publicKey,
            appendAuthority: payer.publicKey,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
          },
          signers: [payer],
        })
      );
    }
    let txId = await Gummyroll.provider.send(tx, [payer, merkleRollKeypair], {
      commitment: "confirmed",
    });
    if (canopyDepth) {
      await logTx(Gummyroll.provider, txId);
    }

    await assertOnChainMerkleRollProperties(
      Gummyroll.provider.connection,
      maxDepth,
      maxSize,
      payer.publicKey,
      new PublicKey(tree.root),
      merkleRollKeypair.publicKey
    );

    return [merkleRollKeypair, tree];
  }

  beforeEach(async () => {
    payer = Keypair.generate();
    connection = new web3Connection("http://localhost:8899", {
      commitment: "confirmed",
    });
    wallet = new NodeWallet(payer);
    anchor.setProvider(
      new Provider(connection, wallet, {
        commitment: connection.commitment,
        skipPreflight: true,
      })
    );
    Gummyroll = anchor.workspace.Gummyroll as Program<Gummyroll>;

    await Gummyroll.provider.connection.confirmTransaction(
      await Gummyroll.provider.connection.requestAirdrop(payer.publicKey, 1e10),
      "confirmed"
    );
  });

  describe("Having created a tree with a single leaf", () => {
    beforeEach(async () => {
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(payer, 1);
    });
    it("Append single leaf", async () => {
      const newLeaf = crypto.randomBytes(32);
      const appendIx = createAppendIx(
        Gummyroll,
        newLeaf,
        payer,
        payer,
        merkleRollKeypair.publicKey
      );

      await execute(Gummyroll.provider, [appendIx], [payer]);

      updateTree(offChainTree, newLeaf, 1);

      const merkleRollAccount =
        await Gummyroll.provider.connection.getAccountInfo(
          merkleRollKeypair.publicKey
        );
      const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
      const onChainRoot =
        merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root matches root of updated off chain tree"
      );
    });
    it("Verify proof works for that leaf", async () => {
      const previousLeaf = offChainTree.leaves[0].node;
      const newLeaf = crypto.randomBytes(32);
      const index = 0;
      const proof = getProofOfLeaf(offChainTree, index).map((treeNode) => {
        return treeNode.node;
      });

      const verifyLeafIx = createVerifyLeafIx(
        Gummyroll,
        merkleRollKeypair.publicKey,
        offChainTree.root,
        previousLeaf,
        index,
        proof
      );
      const replaceLeafIx = createReplaceIx(
        Gummyroll,
        payer,
        merkleRollKeypair.publicKey,
        offChainTree.root,
        previousLeaf,
        newLeaf,
        index,
        proof
      );
      await execute(Gummyroll.provider, [verifyLeafIx, replaceLeafIx], [payer]);

      updateTree(offChainTree, newLeaf, index);

      const merkleRollAccount =
        await Gummyroll.provider.connection.getAccountInfo(
          merkleRollKeypair.publicKey
        );
      const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
      const onChainRoot =
        merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root matches root of updated off chain tree"
      );
    });
    it("Verify leaf fails when proof fails", async () => {
      const previousLeaf = offChainTree.leaves[0].node;
      const newLeaf = crypto.randomBytes(32);
      const index = 0;
      // Proof has random bytes: definitely wrong
      const proof = getProofOfLeaf(offChainTree, index).map((treeNode) => {
        return crypto.randomBytes(32);
      });

      // Verify proof is invalid
      const verifyLeafIx = createVerifyLeafIx(
        Gummyroll,
        merkleRollKeypair.publicKey,
        offChainTree.root,
        previousLeaf,
        index,
        proof
      );
      try {
        await execute(Gummyroll.provider, [verifyLeafIx], [payer]);
        assert(false, "Proof should have failed to verify");
      } catch { }

      // Replace instruction with same proof fails
      const replaceLeafIx = createReplaceIx(
        Gummyroll,
        payer,
        merkleRollKeypair.publicKey,
        offChainTree.root,
        previousLeaf,
        newLeaf,
        index,
        proof
      );
      try {
        await execute(Gummyroll.provider, [replaceLeafIx], [payer]);
        assert(false, "Replace should have failed to verify");
      } catch { }
      const merkleRollAccount =
        await Gummyroll.provider.connection.getAccountInfo(
          merkleRollKeypair.publicKey
        );
      const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
      const onChainRoot =
        merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root matches root of updated off chain tree"
      );
    });
    it("Replace that leaf", async () => {
      const previousLeaf = offChainTree.leaves[0].node;
      const newLeaf = crypto.randomBytes(32);
      const index = 0;

      const replaceLeafIx = createReplaceIx(
        Gummyroll,
        payer,
        merkleRollKeypair.publicKey,
        offChainTree.root,
        previousLeaf,
        newLeaf,
        index,
        getProofOfLeaf(offChainTree, index, false, -1).map((treeNode) => {
          return treeNode.node;
        })
      );
      assert(
        replaceLeafIx.keys.length == 3 + MAX_DEPTH,
        `Failed to create proof for ${MAX_DEPTH}`
      );

      await execute(Gummyroll.provider, [replaceLeafIx], [payer]);

      updateTree(offChainTree, newLeaf, index);

      const merkleRollAccount =
        await Gummyroll.provider.connection.getAccountInfo(
          merkleRollKeypair.publicKey
        );
      const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
      const onChainRoot =
        merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root matches root of updated off chain tree"
      );
    });

    it("Replace that leaf with a minimal proof", async () => {
      const previousLeaf = offChainTree.leaves[0].node;
      const newLeaf = crypto.randomBytes(32);
      const index = 0;

      const replaceLeafIx = createReplaceIx(
        Gummyroll,
        payer,
        merkleRollKeypair.publicKey,
        offChainTree.root,
        previousLeaf,
        newLeaf,
        index,
        getProofOfLeaf(offChainTree, index, true, 1).map((treeNode) => {
          return treeNode.node;
        })
      );
      assert(
        replaceLeafIx.keys.length == 3 + 1,
        "Failed to minimize proof to expected size of 1"
      );
      await execute(Gummyroll.provider, [replaceLeafIx], [payer]);

      updateTree(offChainTree, newLeaf, index);

      const merkleRollAccount =
        await Gummyroll.provider.connection.getAccountInfo(
          merkleRollKeypair.publicKey
        );
      const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
      const onChainRoot =
        merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root matches root of updated off chain tree"
      );
    });
  });

  describe("Examples tranferring appendAuthority", () => {
    const authority = Keypair.generate();
    const randomSigner = Keypair.generate();
    describe("Examples transferring appendAuthority", () => {
      it("... initializing tree ...", async () => {
        await Gummyroll.provider.connection.confirmTransaction(
          await (connection as Connection).requestAirdrop(
            authority.publicKey,
            1e10
          )
        );
        [merkleRollKeypair, offChainTree] = await createTreeOnChain(
          authority,
          1
        );
      });
      it("Attempting to append without appendAuthority fails", async () => {
        // Random leaf
        const newLeaf = crypto.randomBytes(32);
        const appendIx = createAppendIx(
          Gummyroll,
          newLeaf,
          authority,
          randomSigner,
          merkleRollKeypair.publicKey
        );

        try {
          await execute(Gummyroll.provider, [appendIx], [payer, randomSigner]);
          assert(
            false,
            "Transaction should have failed, since `randomSigner` is not append authority"
          );
        } catch { }
      });
      it("But authority can transfer appendAuthority", async () => {
        const transferAppendAuthorityIx = createTransferAuthorityIx(
          Gummyroll,
          authority,
          merkleRollKeypair.publicKey,
          null,
          randomSigner.publicKey
        );
        await execute(
          Gummyroll.provider,
          [transferAppendAuthorityIx],
          [authority]
        );

        const merkleRoll = decodeMerkleRoll(
          (
            await Gummyroll.provider.connection.getAccountInfo(
              merkleRollKeypair.publicKey
            )
          ).data
        );
        const merkleRollInfo = merkleRoll.header;

        assert(
          merkleRollInfo.authority.equals(authority.publicKey),
          `Upon transfering appendAuthority, authority should be ${authority.publicKey.toString()}, but was instead updated to ${merkleRollInfo.authority.toString()}`
        );
        assert(
          merkleRollInfo.appendAuthority.equals(randomSigner.publicKey),
          `Upon transferring appendAuthority, appendAuthority should be ${randomSigner.publicKey.toString()} but is ${merkleRollInfo.appendAuthority.toString()}`
        );
      });
      it("So the new appendAuthority can append", async () => {
        const newLeaf = crypto.randomBytes(32);
        const appendIx = createAppendIx(
          Gummyroll,
          newLeaf,
          authority,
          randomSigner,
          merkleRollKeypair.publicKey
        );
        await execute(
          Gummyroll.provider,
          [appendIx],
          [authority, randomSigner]
        );

        const merkleRoll = decodeMerkleRoll(
          (
            await Gummyroll.provider.connection.getAccountInfo(
              merkleRollKeypair.publicKey
            )
          ).data
        );
        assert(
          merkleRoll.roll.rightMostPath.index === 2,
          `Expected merkle roll to now have 2 leaves after append, but only has ${merkleRoll.roll.rightMostPath.index}`
        );

        updateTree(offChainTree, newLeaf, 1);
      });
      it("but not replace", async () => {
        const newLeaf = crypto.randomBytes(32);
        const replaceIx = createReplaceIx(
          Gummyroll,
          randomSigner,
          merkleRollKeypair.publicKey,
          offChainTree.root,
          offChainTree.leaves[1].node,
          newLeaf,
          1,
          getProofOfLeaf(offChainTree, 1).map((treeNode) => {
            return treeNode.node;
          })
        );
        try {
          await execute(Gummyroll.provider, [replaceIx], [randomSigner]);
          assert(
            false,
            "Transaction should have failed since the append authority cannot act as the authority for replaces"
          );
        } catch { }
      });
    });
    describe("Examples transferring authority", () => {
      it("... initializing tree ...", async () => {
        await Gummyroll.provider.connection.confirmTransaction(
          await (connection as Connection).requestAirdrop(
            authority.publicKey,
            1e10
          )
        );
        [merkleRollKeypair, offChainTree] = await createTreeOnChain(
          authority,
          1
        );
      });
      it("Attempting to append without appendAuthority fails", async () => {
        await (connection as Connection).requestAirdrop(
          randomSigner.publicKey,
          1e10
        );

        const newLeaf = crypto.randomBytes(32);
        const replaceIndex = 0;
        const proof = getProofOfLeaf(offChainTree, replaceIndex);
        const replaceIx = createReplaceIx(
          Gummyroll,
          randomSigner,
          merkleRollKeypair.publicKey,
          offChainTree.root,
          offChainTree.leaves[replaceIndex].node,
          newLeaf,
          replaceIndex,
          proof.map((treeNode) => {
            return treeNode.node;
          })
        );

        try {
          await execute(Gummyroll.provider, [replaceIx], [randomSigner]);
          assert(
            false,
            "Transaction should have failed since incorrect authority cannot execute replaces"
          );
        } catch { }
      });
      it("Can transfer authority", async () => {
        const transferAppendAuthorityIx = createTransferAuthorityIx(
          Gummyroll,
          authority,
          merkleRollKeypair.publicKey,
          randomSigner.publicKey,
          null
        );
        await execute(
          Gummyroll.provider,
          [transferAppendAuthorityIx],
          [authority]
        );

        const merkleRoll = decodeMerkleRoll(
          (
            await Gummyroll.provider.connection.getAccountInfo(
              merkleRollKeypair.publicKey
            )
          ).data
        );
        const merkleRollInfo = merkleRoll.header;

        assert(
          merkleRollInfo.authority.equals(randomSigner.publicKey),
          `Upon transfering appendAuthority, authority should be ${randomSigner.publicKey.toString()}, but was instead updated to ${merkleRollInfo.authority.toString()}`
        );
        assert(
          merkleRollInfo.appendAuthority.equals(authority.publicKey),
          `Upon transferring appendAuthority, appendAuthority should be ${authority.publicKey.toString()} but is ${merkleRollInfo.appendAuthority.toString()}`
        );
      });
      it("Attempting to replace with new authority now works", async () => {
        const newLeaf = crypto.randomBytes(32);
        const replaceIndex = 0;
        const proof = getProofOfLeaf(offChainTree, replaceIndex);
        const replaceIx = createReplaceIx(
          Gummyroll,
          randomSigner,
          merkleRollKeypair.publicKey,
          offChainTree.root,
          offChainTree.leaves[replaceIndex].node,
          newLeaf,
          replaceIndex,
          proof.map((treeNode) => {
            return treeNode.node;
          })
        );

        try {
          await execute(Gummyroll.provider, [replaceIx], [randomSigner]);
          assert(
            false,
            "Transaction should have failed since incorrect authority cannot execute replaces"
          );
        } catch { }
      });
    });
  });

  describe(`Having created a tree with ${MAX_SIZE} leaves`, () => {
    beforeEach(async () => {
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(
        payer,
        MAX_SIZE
      );
    });
    it(`Replace all of them in a block`, async () => {
      // Replace 64 leaves before syncing off-chain tree with on-chain tree

      // Cache all proofs so we can execute in single block
      let ixArray = [];
      let txList = [];

      const leavesToUpdate = [];
      for (let i = 0; i < MAX_SIZE; i++) {
        const index = i;
        const newLeaf = hash(
          payer.publicKey.toBuffer(),
          Buffer.from(new BN(i).toArray())
        );
        leavesToUpdate.push(newLeaf);
        const proof = getProofOfLeaf(offChainTree, index);
        const replaceIx = createReplaceIx(
          Gummyroll,
          payer,
          merkleRollKeypair.publicKey,
          offChainTree.root,
          offChainTree.leaves[i].node,
          newLeaf,
          index,
          proof.map((treeNode) => {
            return treeNode.node;
          })
        );
        ixArray.push(replaceIx);
      }

      // Execute all replaces in a "single block"
      ixArray.map((ix) => {
        const tx = new Transaction().add(ix);
        txList.push(
          Gummyroll.provider.send(tx, [payer], {
            commitment: "confirmed",
            skipPreflight: true,
          })
        );
      });
      await Promise.all(txList);

      leavesToUpdate.map((leaf, index) => {
        updateTree(offChainTree, leaf, index);
      });

      // Compare on-chain & off-chain roots
      const merkleRoll = decodeMerkleRoll(
        (
          await Gummyroll.provider.connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );
      const onChainRoot =
        merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root does not match root of updated off chain tree"
      );
    });
  });

  describe(`Having created a tree with depth 3`, () => {
    const DEPTH = 3;
    beforeEach(async () => {
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(
        payer,
        0,
        DEPTH,
        2 ** DEPTH
      );

      for (let i = 0; i < 2 ** DEPTH; i++) {
        const newLeaf = Array.from(Buffer.alloc(32, i + 1));
        const appendIx = createAppendIx(
          Gummyroll,
          newLeaf,
          payer,
          payer,
          merkleRollKeypair.publicKey
        );
        await execute(Gummyroll.provider, [appendIx], [payer]);
      }

      // Compare on-chain & off-chain roots
      const merkleRoll = decodeMerkleRoll(
        (
          await Gummyroll.provider.connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );

      assert(
        merkleRoll.roll.bufferSize === 2 ** DEPTH,
        "Not all changes were processed"
      );
      assert(
        merkleRoll.roll.activeIndex === 0,
        "Not all changes were processed"
      );
    });

    it("Random attacker fails to fake the existence of a leaf by autocompleting proof", async () => {
      const maliciousLeafHash = crypto.randomBytes(32);
      const maliciousLeafHash1 = crypto.randomBytes(32);
      const nodeProof = [];
      for (let i = 0; i < DEPTH; i++) {
        nodeProof.push(Buffer.alloc(32));
      }

      // Root - make this nonsense so it won't match what's in CL, and force proof autocompletion
      const replaceIx = createReplaceIx(
        Gummyroll,
        payer,
        merkleRollKeypair.publicKey,
        Buffer.alloc(32),
        maliciousLeafHash,
        maliciousLeafHash1,
        0,
        nodeProof
      );

      try {
        await execute(Gummyroll.provider, [replaceIx], [payer]);
        assert(
          false,
          "Attacker was able to succesfully write fake existence of a leaf"
        );
      } catch (e) { }

      const merkleRoll = decodeMerkleRoll(
        (
          await Gummyroll.provider.connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );

      assert(
        merkleRoll.roll.activeIndex === 0,
        "Merkle roll updated its active index after attacker's transaction, when it shouldn't have done anything"
      );
    });
    it("Random attacker fails to fake the existence of a leaf by autocompleting proof", async () => {
      const maliciousLeafHash = crypto.randomBytes(32);
      const maliciousLeafHash1 = crypto.randomBytes(32);
      const nodeProof = [];
      for (let i = 0; i < DEPTH; i++) {
        nodeProof.push(Buffer.alloc(32));
      }

      // Root - make this nonsense so it won't match what's in CL, and force proof autocompletion
      const replaceIx = createReplaceIx(
        Gummyroll,
        payer,
        merkleRollKeypair.publicKey,
        Buffer.alloc(32),
        maliciousLeafHash,
        maliciousLeafHash1,
        0,
        nodeProof
      );

      try {
        await execute(Gummyroll.provider, [replaceIx], [payer]);
        assert(
          false,
          "Attacker was able to succesfully write fake existence of a leaf"
        );
      } catch (e) { }

      const merkleRoll = decodeMerkleRoll(
        (
          await Gummyroll.provider.connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );

      assert(
        merkleRoll.roll.activeIndex === 0,
        "Merkle roll updated its active index after attacker's transaction, when it shouldn't have done anything"
      );
    });
  });
  describe(`Canopy test`, () => {
    const DEPTH = 5;
    it("Testing canopy for appends and replaces on a full on chain tree", async () => {
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(
        payer,
        0,
        DEPTH,
        8,
        DEPTH // Store full tree on chain
      );

      let leaves = [];
      let i = 0;
      let stepSize = 4;
      while (i < 2 ** DEPTH) {
        let ixs = [];
        for (let j = 0; j < stepSize; ++j) {
          const newLeaf = Array.from(Buffer.alloc(32, i + 1));
          leaves.push(newLeaf);
          const appendIx = createAppendIx(
            Gummyroll,
            newLeaf,
            payer,
            payer,
            merkleRollKeypair.publicKey
          );
          ixs.push(appendIx);
        }
        await execute(Gummyroll.provider, ixs, [payer]);
        i += stepSize;
        console.log("Appended", i, "leaves");
      }

      // Compare on-chain & off-chain roots
      let ixs = [];
      const merkleRoll = decodeMerkleRoll(
        (
          await Gummyroll.provider.connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );

      let root = merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root;
      let leafList = Array.from(leaves.entries());
      leafList.sort(() => Math.random() - 0.5);
      let replaces = 0;
      for (const [i, leaf] of leafList) {
        const newLeaf = crypto.randomBytes(32);
        const replaceIx = createReplaceIx(
          Gummyroll,
          payer,
          merkleRollKeypair.publicKey,
          root.toBuffer(),
          leaf,
          newLeaf,
          i,
          [] // No proof necessary
        );
        ixs.push(replaceIx);
        if (ixs.length == stepSize) {
          replaces++;
          let tx = await execute(Gummyroll.provider, ixs, [payer]);
          console.log("Replaced", replaces * stepSize, "leaves");
          ixs = [];
        }
      }
    });
  });
});
