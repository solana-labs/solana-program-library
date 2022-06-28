import express from "express";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { bootstrap, NFTDatabaseConnection, Proof } from "./db";
import cors from 'cors';

const app = express();
app.use(cors())
app.use(express.json());

let nftDb: NFTDatabaseConnection;
const port = 4000;

type JsonProof = {
  root: String;
  proofNodes: String[];
  leaf: String;
  index: number;
  nonce: number;
  dataHash: string;
  creatorHash: string;
  owner: string;
  delegate: string;
};

function stringifyProof(proof: Proof): string {
  let jsonProof: JsonProof = {
    root: proof.root,
    proofNodes: proof.proofNodes,
    leaf: proof.leaf,
    index: proof.index,
    nonce: proof.nonce,
    dataHash: proof.dataHash,
    creatorHash: proof.creatorHash,
    owner: proof.owner,
    delegate: proof.delegate,
  };
  return JSON.stringify(jsonProof);
}

app.get("/proof", async (req, res) => {
  const leafHashString = req.query.leafHash;
  const treeId = req.query.treeId;
  const leafHash: Buffer = bs58.decode(leafHashString);
  try {
    let proof = await nftDb.getInferredProof(leafHash, treeId, false);
    if (proof) {
      res.send(stringifyProof(proof));
    } else {
      res.send(JSON.stringify({ err: "Failed to fetch proof" }));
    }
  } catch (e) {
    res.send(
      JSON.stringify({ err: `Encounter error while fetching proof: ${e}` })
    );
  }
});

app.get("/assets", async (req, res) => {
  const owner = req.query.owner;
  const assets = await nftDb.getAssetsForOwner(owner, req.query.treeId);
  res.send(JSON.stringify(assets));
});

app.listen(port, async () => {
  nftDb = await bootstrap(false);
  console.log(`Tree server listening on port ${port}`);
});
