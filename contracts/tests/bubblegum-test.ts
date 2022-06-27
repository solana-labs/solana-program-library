import * as anchor from "@project-serum/anchor";
import {keccak_256} from "js-sha3";
import {BN, Provider, Program} from "@project-serum/anchor";
import {Bubblegum} from "../target/types/bubblegum";
import {Gummyroll} from "../target/types/gummyroll";
import {PROGRAM_ID} from "@metaplex-foundation/mpl-token-metadata";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  SYSVAR_RENT_PUBKEY,
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

import {buildTree, Tree} from "./merkle-tree";
import {
  decodeMerkleRoll,
  getMerkleRollAccountSize,
  getRootOfOnChainMerkleRoot,
  assertOnChainMerkleRollProperties,
  createTransferAuthorityIx,
} from "../sdk/gummyroll";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  Token,
} from "@solana/spl-token";
import { execute, logTx, bufferToArray } from "./utils";
import { TokenProgramVersion, Version } from "../sdk/bubblegum/src/generated";

// @ts-ignore
let Bubblegum;
// @ts-ignore
let GummyrollProgramId;

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
      await Bubblegum.provider.connection.requestAirdrop(payer.publicKey, 2e9),
      "confirmed"
    );
    await Bubblegum.provider.connection.confirmTransaction(
      await Bubblegum.provider.connection.requestAirdrop(
        destination.publicKey,
        2e9
      ),
      "confirmed"
    );
    await Bubblegum.provider.connection.confirmTransaction(
      await Bubblegum.provider.connection.requestAirdrop(
        delegate.publicKey,
        2e9
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
      const metadata = {
        name: "test",
        symbol: "test",
        uri: "www.solana.com",
        sellerFeeBasisPoints: 0,
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
          gummyrollProgram: GummyrollProgramId,
          owner: payer.publicKey,
          delegate: payer.publicKey,
          merkleSlab: merkleRollKeypair.publicKey,
        },
        {message: metadata}
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
      const dataHash = bufferToArray(
        Buffer.from(keccak_256.digest(mintIx.data.slice(8)))
      );
      const creatorHash = bufferToArray(Buffer.from(keccak_256.digest([])));
      let onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);
      
        console.log(" - Transferring Ownership");
      const nonceInfo = await (
        Bubblegum.provider.connection as web3Connection
      ).getAccountInfo(treeAuthority);
      const leafNonce = new BN(nonceInfo.data.slice(8, 16), "le").sub(
        new BN(1)
      );
      let transferIx = createTransferInstruction(
        {
          authority: treeAuthority,
          owner: payer.publicKey,
          delegate: payer.publicKey,
          newOwner: destination.publicKey,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
        },
        {
          root: bufferToArray(onChainRoot),
          dataHash,
          creatorHash,
          nonce: leafNonce,
          index: 0,
        }
      );
      await execute(Bubblegum.provider, [transferIx], [payer]);

      onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);

      console.log(" - Delegating Ownership");
      let delegateIx = await createDelegateInstruction(
        {
          authority: treeAuthority,
          owner: destination.publicKey,
          previousDelegate: destination.publicKey,
          newDelegate: delegateKey.publicKey,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
        },
        {
          root: bufferToArray(onChainRoot),
          dataHash,
          creatorHash,
          nonce: leafNonce,
          index: 0,
        }
      );
      await execute(Bubblegum.provider, [delegateIx], [destination]);

      onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);

      console.log(" - Transferring Ownership (through delegate)");
      let delTransferIx = createTransferInstruction(
        {
          authority: treeAuthority,
          owner: destination.publicKey,
          delegate: delegateKey.publicKey,
          newOwner: payer.publicKey,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
        },
        {
          root: bufferToArray(onChainRoot),
          dataHash,
          creatorHash,
          nonce: leafNonce,
          index: 0,
        }
      );
      delTransferIx.keys[2].isSigner = true;
      let delTransferTx = await Bubblegum.provider.send(
        new Transaction().add(delTransferIx),
        [delegateKey],
        {
          skipPreflight: true,
          commitment: "confirmed",
        }
      );

      onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);

      let [voucher] = await PublicKey.findProgramAddress(
        [
          Buffer.from("voucher", "utf8"),
          merkleRollKeypair.publicKey.toBuffer(),
          new BN(0).toBuffer("le", 8)
        ],
        Bubblegum.programId
      );

      console.log(" - Redeeming Leaf", voucher.toBase58());
      let redeemIx = createRedeemInstruction(
        {
          authority: treeAuthority,
          owner: payer.publicKey,
          delegate: payer.publicKey,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          voucher: voucher,
        },
        {
          root: bufferToArray(onChainRoot),
          dataHash,
          creatorHash,
          nonce: new BN(0),
          index: 0,
        }
      );
      let redeemTx = await Bubblegum.provider.send(
        new Transaction().add(redeemIx),
        [payer],
        {
          skipPreflight: true,
          commitment: "confirmed",
        }
      );
      console.log(" - Cancelling redeem (reinserting to tree)");

      const cancelRedeemIx = createCancelRedeemInstruction(
        {
          authority: treeAuthority,
          owner: payer.publicKey,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          voucher: voucher,
        },
        {
          root: bufferToArray(onChainRoot),
        }
      );
      let cancelRedeemTx = await Bubblegum.provider.send(
        new Transaction().add(cancelRedeemIx),
        [payer],
        {
          commitment: "confirmed",
        }
      );

      console.log(" - Decompressing leaf");

      redeemIx = createRedeemInstruction(
        {
          authority: treeAuthority,
          owner: payer.publicKey,
          delegate: payer.publicKey,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          voucher: voucher,
        },
        {
          root: bufferToArray(onChainRoot),
          dataHash,
          creatorHash,
          nonce: leafNonce,
          index: 0,
        }
      );
      let redeemTx2 = await Bubblegum.provider.send(
        new Transaction().add(redeemIx),
        [payer],
        {
          commitment: "confirmed",
        }
      );

      let voucherData = await Bubblegum.account.voucher.fetch(voucher);

      let [asset] = await PublicKey.findProgramAddress(
        [Buffer.from("asset"), merkleRollKeypair.publicKey.toBuffer(), leafNonce.toBuffer("le", 8)],
        Bubblegum.programId
      );

      let [tokenMint] = await PublicKey.findProgramAddress(
        [asset.toBuffer(), TOKEN_PROGRAM_ID.toBuffer()],
        Bubblegum.programId
      ); //TODO -> change this for v1 id must be the mint

      let [mintAuthority] = await PublicKey.findProgramAddress(
        [tokenMint.toBuffer()],
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
            tokenMint,
            payer.publicKey
          ),
          mint: tokenMint,
          mintAuthority: mintAuthority,
          metadata: await getMetadata(tokenMint),
          masterEdition: await getMasterEdition(tokenMint),
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
    });
  });
});
