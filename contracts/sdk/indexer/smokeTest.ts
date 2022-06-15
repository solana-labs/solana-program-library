import { Program, web3 } from "@project-serum/anchor";
import fetch from "node-fetch";
import { getMerkleRollAccountSize, Gummyroll } from "../gummyroll";
import * as anchor from "@project-serum/anchor";
import {
  AccountMeta,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { Bubblegum } from "../../target/types/bubblegum";
import {
  createCreateTreeInstruction,
  createMintInstruction,
  createTransferInstruction,
  TokenProgramVersion,
  Version,
} from "../bubblegum/src/generated";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { logTx } from "../../tests/utils";

async function main() {
  const connection = new web3.Connection("http://127.0.0.1:8899", {
    commitment: "confirmed",
  });
  const proofServerUrl = "http://127.0.0.1:4000/proof";
  const assetServerUrl = "http://127.0.0.1:4000/assets";
  const payer = Keypair.generate();
  const wallet = new NodeWallet(payer);
  anchor.setProvider(
    new anchor.Provider(connection, wallet, {
      commitment: connection.commitment,
      skipPreflight: true,
    })
  );
  let GummyrollCtx = anchor.workspace.Gummyroll as Program<Gummyroll>;
  let BubblegumCtx = anchor.workspace.Bubblegum as Program<Bubblegum>;
  await BubblegumCtx.provider.connection.confirmTransaction(
    await BubblegumCtx.provider.connection.requestAirdrop(
      payer.publicKey,
      1e10
    ),
    "confirmed"
  );

  let wallets = [];
  for (let i = 0; i < 20; ++i) {
    const spaces = "                                         ";
    wallets.push(
      Keypair.fromSeed(
        Uint8Array.from(Buffer.from(`bubblegum${i}${spaces}`.slice(0, 32)))
      )
    );
    console.log(i, bs58.encode(wallets[i].secretKey));
    if (
      (await BubblegumCtx.provider.connection.getBalance(
        wallets[i].publicKey
      )) > 0
    ) {
      continue;
    }
    await BubblegumCtx.provider.connection.confirmTransaction(
      await BubblegumCtx.provider.connection.sendTransaction(
        new Transaction().add(
          SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: wallets[i].publicKey,
            lamports: 100000000,
          })
        ),
        [payer]
      ),
      "confirmed"
    );
  }

  let maxDepth = 20;
  let maxSize = 1024;
  const merkleRollKeypair = Keypair.generate();
  const requiredSpace = getMerkleRollAccountSize(maxDepth, maxSize);
  const allocAccountIx = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: merkleRollKeypair.publicKey,
    lamports:
      await BubblegumCtx.provider.connection.getMinimumBalanceForRentExemption(
        requiredSpace
      ),
    space: requiredSpace,
    programId: GummyrollCtx.programId,
  });

  let [authority] = await PublicKey.findProgramAddress(
    [merkleRollKeypair.publicKey.toBuffer()],
    BubblegumCtx.programId
  );
  let [nonce] = await PublicKey.findProgramAddress(
    [Buffer.from("bubblegum"), merkleRollKeypair.publicKey.toBuffer()],
    BubblegumCtx.programId
  );
  let createTreeIx = createCreateTreeInstruction(
    {
      treeCreator: payer.publicKey,
      payer: payer.publicKey,
      authority: authority,
      gummyrollProgram: GummyrollCtx.programId,
      merkleSlab: merkleRollKeypair.publicKey,
      nonce: nonce,
    },
    {
      maxDepth,
      maxBufferSize: maxSize,
    }
  );
  let tx = new Transaction();
  tx = tx.add(allocAccountIx).add(createTreeIx);
  let txId = await BubblegumCtx.provider.connection.sendTransaction(
    tx,
    [payer, merkleRollKeypair],
    {
      skipPreflight: true,
    }
  );
  await logTx(BubblegumCtx.provider, txId);
  let numMints = 0;
  while (1) {
    let i = Math.floor(Math.random() * wallets.length);
    let j = Math.floor(Math.random() * wallets.length);
    if (i === j) {
      continue;
    }
    if (Math.random() < 0.5) {
      let tx = new Transaction().add(
        createMintInstruction(
          {
            mintAuthority: payer.publicKey,
            authority: authority,
            merkleSlab: merkleRollKeypair.publicKey,
            gummyrollProgram: GummyrollCtx.programId,
            owner: wallets[i].publicKey,
            delegate: wallets[i].publicKey,
            nonce: nonce,
          },
          {
            version: Version.V0,
            message: {
              name: `BUBBLE #${numMints}`,
              symbol: "BUBBLE",
              uri: Keypair.generate().publicKey.toBase58(),
              sellerFeeBasisPoints: 100,
              primarySaleHappened: true,
              isMutable: true,
              editionNonce: null,
              tokenStandard: null,
              collection: null,
              uses: null,
              tokenProgramVersion: TokenProgramVersion.Original,
              creators: [
                { address: payer.publicKey, share: 100, verified: true },
              ],
            },
          }
        )
      );
      await BubblegumCtx.provider.connection.sendTransaction(tx, [
        payer,
        wallets[i],
      ]);
      numMints++;
    } else {
      let response = await fetch(
        `${assetServerUrl}?owner=${wallets[i].publicKey.toBase58()}`,
        { method: "GET" }
      );
      const assets = await response.json();
      if (assets.length === 0) {
        continue;
      }
      let k = Math.floor(Math.random() * assets.length);
      response = await fetch(
        `${proofServerUrl}?leafHash=${assets[k].leafHash}&treeId=${assets[k].treeId}`,
        { method: "GET" }
      );
      const proof = await response.json();
      const proofNodes: Array<AccountMeta> = proof.proofNodes.map((key) => {
        return {
          pubkey: new PublicKey(key),
          isWritable: false,
          isSigner: false,
        };
      });
      let [merkleAuthority] = await PublicKey.findProgramAddress(
        [bs58.decode(assets[k].treeId)],
        BubblegumCtx.programId
      );
      let replaceIx = createTransferInstruction(
        {
          owner: wallets[i].publicKey,
          delegate: new PublicKey(proof.delegate),
          newOwner: wallets[j].publicKey,
          authority: merkleAuthority,
          merkleSlab: new PublicKey(assets[k].treeId),
          gummyrollProgram: GummyrollCtx.programId,
        },
        {
          version: Version.V0,
          dataHash: [...bs58.decode(proof.dataHash)],
          creatorHash: [...bs58.decode(proof.creatorHash)],
          root: [...bs58.decode(proof.root)],
          nonce: proof.nonce,
          index: proof.index,
        }
      );
      replaceIx.keys[1].isSigner = true;
      replaceIx.keys = [...replaceIx.keys, ...proofNodes];
      let tx = new Transaction().add(replaceIx);
      await BubblegumCtx.provider.connection
        .sendTransaction(tx, [wallets[i]])
        .then(() =>
          console.log(
            `Successfully transferred asset (${assets[k].leafHash} from tree: ${
              assets[k].treeId
            }) - ${wallets[i].publicKey.toBase58()} -> ${wallets[
              j
            ].publicKey.toBase58()}`
          )
        )
        .catch((e) => console.log("Encountered Error when transferring", e));
    }
  }
}

main()
  .then(() => {
    console.log("Done");
  })
  .catch((e) => {
    console.error(e);
  });
