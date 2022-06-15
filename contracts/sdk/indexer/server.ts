import express from "express";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { bootstrap, Proof } from "./db";

const app = express();
app.use(express.json());

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
  console.log("GET request:", leafHashString);
  const nftDb = await bootstrap(false);
  const leafHash: Buffer = bs58.decode(leafHashString);
  const proof = await nftDb.getProof(leafHash, treeId, false);
  res.send(stringifyProof(proof));
});

app.get("/assets", async (req, res) => {
  const owner = req.query.owner;
  console.log("GET request:", owner);
  const nftDb = await bootstrap(false);
  const assets = await nftDb.getAssetsForOwner(owner);
  res.send(JSON.stringify(assets));
});

app.listen(port, () => {
  console.log(`Example app listening on port ${port}`);
});
