import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { BN } from 'bn.js';
import { AnchorProvider } from "@project-serum/anchor";
import {
  Connection,
  Signer,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { assert } from "chai";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import * as crypto from "crypto";

import {
  createTreeOnChain,
  execute
} from './utils';
import {
  buildTree,
  hash,
  getProofOfLeaf,
  updateTree,
  Tree,
} from "./merkleTree";
import {
  createReplaceIx,
  createAppendIx,
  createTransferAuthorityIx,
  createVerifyLeafIx,
  ConcurrentMerkleTreeAccount,
} from "../src";

describe("Account Compression", () => {
  // Configure the client to use the local cluster.
  let offChainTree: Tree;
  let cmtKeypair: Keypair;
  let payer: Keypair;
  let connection: Connection;
  let provider: AnchorProvider;

  const MAX_SIZE = 64;
  const MAX_DEPTH = 14;


  beforeEach(async () => {
    payer = Keypair.generate();
    connection = new Connection("http://localhost:8899", {
      commitment: "confirmed",
    });
    const wallet = new NodeWallet(payer);
    provider = new AnchorProvider(connection, wallet, {
      commitment: connection.commitment,
      skipPreflight: true,
    });

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, 1e10),
      "confirmed"
    );
  });

  describe("Having created a tree with a single leaf", () => {
    beforeEach(async () => {
      [cmtKeypair, offChainTree] = await createTreeOnChain(provider, payer, 1, MAX_DEPTH, MAX_SIZE);
    });
    it("Append single leaf", async () => {
      const newLeaf = crypto.randomBytes(32);
      const appendIx = createAppendIx(
        newLeaf,
        payer,
        cmtKeypair.publicKey
      );

      await execute(provider, [appendIx], [payer])
      updateTree(offChainTree, newLeaf, 1);

      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);
      const onChainRoot = splCMT.getCurrentRoot();

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
        cmtKeypair.publicKey,
        offChainTree.root,
        previousLeaf,
        index,
        proof
      );
      const replaceLeafIx = createReplaceIx(
        payer,
        cmtKeypair.publicKey,
        offChainTree.root,
        previousLeaf,
        newLeaf,
        index,
        proof
      );
      await execute(provider, [verifyLeafIx, replaceLeafIx], [payer]);

      updateTree(offChainTree, newLeaf, index);

      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);
      const onChainRoot = splCMT.getCurrentRoot();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root matches root of updated off chain tree"
      )
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
        cmtKeypair.publicKey,
        offChainTree.root,
        previousLeaf,
        index,
        proof
      );
      try {
        await execute(provider, [verifyLeafIx], [payer]);
        assert(false, "Proof should have failed to verify");
      } catch { }

      // Replace instruction with same proof fails
      const replaceLeafIx = createReplaceIx(
        payer,
        cmtKeypair.publicKey,
        offChainTree.root,
        previousLeaf,
        newLeaf,
        index,
        proof
      );
      try {
        await execute(provider, [replaceLeafIx], [payer]);
        assert(false, "Replace should have failed to verify");
      } catch { }

      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);
      const onChainRoot = splCMT.getCurrentRoot();

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
        payer,
        cmtKeypair.publicKey,
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

      await execute(provider, [replaceLeafIx], [payer]);

      updateTree(offChainTree, newLeaf, index);

      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);
      const onChainRoot = splCMT.getCurrentRoot();

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
        payer,
        cmtKeypair.publicKey,
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
      await execute(provider, [replaceLeafIx], [payer]);

      updateTree(offChainTree, newLeaf, index);

      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);
      const onChainRoot = splCMT.getCurrentRoot();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root matches root of updated off chain tree"
      );
    });
  });

  describe("Examples transferring authority", () => {
    const authority = Keypair.generate();
    const randomSigner = Keypair.generate();

    beforeEach(async () => {
      await provider.connection.confirmTransaction(
        await (connection as Connection).requestAirdrop(
          authority.publicKey,
          1e10
        )
      );
      [cmtKeypair, offChainTree] = await createTreeOnChain(
        provider,
        authority,
        1,
        MAX_DEPTH,
        MAX_SIZE
      );
    });
    it("Attempting to replace with random authority fails", async () => {
      const newLeaf = crypto.randomBytes(32);
      const replaceIndex = 0;
      const proof = getProofOfLeaf(offChainTree, replaceIndex);
      const replaceIx = createReplaceIx(
        randomSigner,
        cmtKeypair.publicKey,
        offChainTree.root,
        offChainTree.leaves[replaceIndex].node,
        newLeaf,
        replaceIndex,
        proof.map((treeNode) => {
          return treeNode.node;
        })
      );

      try {
        await execute(provider, [replaceIx], [randomSigner]);
        assert(
          false,
          "Transaction should have failed since incorrect authority cannot execute replaces"
        );
      } catch { }
    });
    it("Can transfer authority", async () => {
      const transferAuthorityIx = createTransferAuthorityIx(
        authority,
        cmtKeypair.publicKey,
        randomSigner.publicKey
      );
      await execute(provider, [transferAuthorityIx], [authority]);

      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);

      assert(
        splCMT.getAuthority().equals(randomSigner.publicKey),
        `Upon transfering authority, authority should be ${randomSigner.publicKey.toString()}, but was instead updated to ${splCMT.getAuthority()}`
      );

      // Attempting to replace with new authority now works
      const newLeaf = crypto.randomBytes(32);
      const replaceIndex = 0;
      const proof = getProofOfLeaf(offChainTree, replaceIndex);
      const replaceIx = createReplaceIx(
        randomSigner,
        cmtKeypair.publicKey,
        offChainTree.root,
        offChainTree.leaves[replaceIndex].node,
        newLeaf,
        replaceIndex,
        proof.map((treeNode) => {
          return treeNode.node;
        })
      );

      await execute(provider, [replaceIx], [randomSigner]);
    });
  });

  describe(`Having created a tree with ${MAX_SIZE} leaves`, () => {
    beforeEach(async () => {
      [cmtKeypair, offChainTree] = await createTreeOnChain(
        provider,
        payer,
        MAX_SIZE,
        MAX_DEPTH,
        MAX_SIZE,
      );
    });
    it(`Replace all of them in a block`, async () => {
      // Replace 64 leaves before syncing off-chain tree with on-chain tree

      // Cache all proofs so we can execute in single block
      let ixArray: TransactionInstruction[] = [];
      let txList: Promise<string>[] = [];

      const leavesToUpdate: Buffer[] = [];
      for (let i = 0; i < MAX_SIZE; i++) {
        const index = i;
        const newLeaf = hash(
          payer.publicKey.toBuffer(),
          Buffer.from(new BN(i).toArray())
        );
        leavesToUpdate.push(newLeaf);
        const proof = getProofOfLeaf(offChainTree, index);
        const replaceIx = createReplaceIx(
          payer,
          cmtKeypair.publicKey,
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
        txList.push(
          execute(provider, [ix], [payer])
        );
      });
      await Promise.all(txList);

      leavesToUpdate.map((leaf, index) => {
        updateTree(offChainTree, leaf, index);
      });

      // Compare on-chain & off-chain roots
      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);
      const onChainRoot = splCMT.getCurrentRoot();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root does not match root of updated off chain tree"
      );
    });
  });

  describe(`Having created a tree with depth 3`, () => {
    const DEPTH = 3;
    beforeEach(async () => {
      [cmtKeypair, offChainTree] = await createTreeOnChain(
        provider,
        payer,
        0,
        DEPTH,
        2 ** DEPTH
      );

      for (let i = 0; i < 2 ** DEPTH; i++) {
        const newLeaf = Array.from(Buffer.alloc(32, i + 1));
        const appendIx = createAppendIx(
          newLeaf,
          payer,
          cmtKeypair.publicKey
        );
        await execute(provider, [appendIx], [payer]);
      }

      // Compare on-chain & off-chain roots
      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);

      assert(
        splCMT.getBufferSize() === 2 ** DEPTH,
        "Not all changes were processed"
      );
      assert(
        splCMT.getActiveIndex() === 0,
        "Not all changes were processed"
      );
    });

    it("Random attacker fails to fake the existence of a leaf by autocompleting proof", async () => {
      const maliciousLeafHash = crypto.randomBytes(32);
      const maliciousLeafHash1 = crypto.randomBytes(32);
      const nodeProof: Buffer[] = [];
      for (let i = 0; i < DEPTH; i++) {
        nodeProof.push(Buffer.alloc(32));
      }

      // Root - make this nonsense so it won't match what's in ChangeLog, thus forcing proof autocompletion
      const replaceIx = createReplaceIx(
        payer,
        cmtKeypair.publicKey,
        Buffer.alloc(32),
        maliciousLeafHash,
        maliciousLeafHash1,
        0,
        nodeProof
      );

      try {
        await execute(provider, [replaceIx], [payer]);
        assert(
          false,
          "Attacker was able to succesfully write fake existence of a leaf"
        );
      } catch (e) { }

      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);

      assert(
        splCMT.getActiveIndex() === 0,
        "CMT updated its active index after attacker's transaction, when it shouldn't have done anything"
      );
    });
    it("Random attacker fails to fake the existence of a leaf by autocompleting proof", async () => {
      // As an attacker, we want to set `maliciousLeafHash1` by 
      // providing `maliciousLeafHash` and `nodeProof` which hash to the current merkle tree root.
      // If we can do this, then we can set leaves to arbitrary values.
      const maliciousLeafHash = crypto.randomBytes(32);
      const maliciousLeafHash1 = crypto.randomBytes(32);
      const nodeProof: Buffer[] = [];
      for (let i = 0; i < DEPTH; i++) {
        nodeProof.push(Buffer.alloc(32));
      }

      // Root - make this nonsense so it won't match what's in CL, and force proof autocompletion
      const replaceIx = createReplaceIx(
        payer,
        cmtKeypair.publicKey,
        Buffer.alloc(32),
        maliciousLeafHash,
        maliciousLeafHash1,
        0,
        nodeProof
      );

      try {
        await execute(provider, [replaceIx], [payer]);
        assert(
          false,
          "Attacker was able to succesfully write fake existence of a leaf"
        );
      } catch (e) { }

      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(provider.connection, cmtKeypair.publicKey);

      assert(
        splCMT.getActiveIndex() === 0,
        "CMT updated its active index after attacker's transaction, when it shouldn't have done anything"
      );
    });
  });
  describe(`Canopy test`, () => {
    const DEPTH = 5;
    it("Testing canopy for appends and replaces on a full on chain tree", async () => {
      [cmtKeypair, offChainTree] = await createTreeOnChain(
        provider,
        payer,
        0,
        DEPTH,
        8,
        DEPTH // Store full tree on chain
      );

      // Test that the canopy updates properly throughout multiple modifying instructions
      // in the same transaction
      let leaves: Array<number>[] = [];
      let i = 0;
      let stepSize = 4;
      while (i < 2 ** DEPTH) {
        let ixs: TransactionInstruction[] = [];
        for (let j = 0; j < stepSize; ++j) {
          const newLeaf = Array.from(Buffer.alloc(32, i + 1));
          leaves.push(newLeaf);
          const appendIx = createAppendIx(
            newLeaf,
            payer,
            cmtKeypair.publicKey
          );
          ixs.push(appendIx);
        }
        await execute(provider, ixs, [payer]);
        i += stepSize;
        console.log("Appended", i, "leaves");
      }

      // Compare on-chain & off-chain roots
      let ixs: TransactionInstruction[] = [];
      const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);
      const root = splCMT.getCurrentRoot();

      // Test that the entire state of the tree is stored properly
      // by using the canopy to infer proofs to all of the leaves in the tree.
      // We test that the canopy is updating properly by replacing all the leaves
      // in the tree
      let leafList = Array.from(leaves.entries());
      leafList.sort(() => Math.random() - 0.5);
      let replaces = 0;
      let newLeaves: Record<number, Buffer> = {};
      for (const [i, leaf] of leafList) {
        const newLeaf = crypto.randomBytes(32);
        newLeaves[i] = newLeaf;
        const replaceIx = createReplaceIx(
          payer,
          cmtKeypair.publicKey,
          root,
          Buffer.from(leaf),
          newLeaf,
          i,
          [] // No proof necessary
        );
        ixs.push(replaceIx);
        if (ixs.length == stepSize) {
          replaces++;
          await execute(provider, ixs, [payer]);
          console.log("Replaced", replaces * stepSize, "leaves");
          ixs = [];
        }
      }

      let newLeafList: Buffer[] = []
      for (let i = 0; i < 32; ++i) {
        newLeafList.push(newLeaves[i])
      }

      let tree = buildTree(newLeafList);

      for (let proofSize = 1; proofSize <= 5; ++proofSize) {
        const newLeaf = crypto.randomBytes(32);
        let i = Math.floor(Math.random() * 32)
        const leaf = newLeaves[i];

        let partialProof = getProofOfLeaf(tree, i).slice(0, proofSize).map((n) => n.node)
        console.log(`Replacing node ${i}, proof length = ${proofSize}`)
        for (const [level, node] of Object.entries(partialProof)) {
          console.log(` ${level}: ${bs58.encode(node)}`)
        }
        const replaceIx = createReplaceIx(
          payer,
          cmtKeypair.publicKey,
          root,
          newLeaves[i],
          newLeaf,
          i,
          partialProof,
        );
        updateTree(tree, newLeaf, i);
        const replaceBackIx = createReplaceIx(
          payer,
          cmtKeypair.publicKey,
          tree.root,
          newLeaf,
          newLeaves[i],
          i,
          partialProof,
        );
        updateTree(tree, leaf, i);
        await execute(provider, [replaceIx, replaceBackIx], [payer], true, true);
      }
    });
  });
});
