require("dotenv").config();

import { Connection, Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { ProgramMetadata } from "../index";
import bs58 from "bs58";
import { expect } from "chai";
import { v4 as uuid } from "uuid";
import { createHash } from "crypto";
import { VersionedIdl } from "../program/accounts/versioned-idl";
import {
  NAME_SERVICE_ACCOUNT_OFFSET,
  NAME_SERVICE_CLASS_OFFSET,
} from "../program/program-metadata";
import BN from "bn.js";

const timeout = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const privateSecretKey = require(process.env.PRIVATE_KEY_PATH || "");
const privateKeypair = Keypair.fromSecretKey(new Uint8Array(privateSecretKey));
const connection = new Connection(process.env.API_URL || "", "single");
const targetProgramKey = new PublicKey(process.env.TARGET_PROGRAM_KEY || "");
const programMetadata = new ProgramMetadata(connection, {
  programMetadataKey: new PublicKey(process.env.PROGRAM_METADATA_KEY || ""),
  nameServiceKey: new PublicKey(process.env.NAME_SERVICE_KEY || ""),
});

describe("ProgramMetadata: MetadataEntry", async () => {
  describe("create, update, delete", async () => {
    let name;

    before(async () => {
      name = uuid();

      const ix = await programMetadata.createMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        name,
        "some metadata"
      );

      const tx = new Transaction();
      const signers: Keypair[] = [];
      signers.push(privateKeypair);
      tx.add(ix);

      await connection.sendTransaction(tx, signers);

      await timeout(5000);
    });

    it("should create a metadata entry", async () => {
      const accts = await connection.getProgramAccounts(
        programMetadata.nameServiceKey,
        {
          filters: [
            {
              memcmp: {
                bytes: bs58.encode(Buffer.from(name)),
                offset: 101,
              },
            },
          ],
        }
      );

      expect(accts.length).to.equal(1);
    });

    it("should update metadata entry", async () => {
      const updateIx = await programMetadata.updateMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        name,
        "new metadata"
      );

      const tx = new Transaction();
      const signers: Keypair[] = [];
      signers.push(privateKeypair);

      tx.add(updateIx);

      const res = await connection.sendTransaction(tx, signers);

      await timeout(5000);

      const accts = await connection.getProgramAccounts(
        programMetadata.nameServiceKey,
        {
          filters: [
            {
              memcmp: {
                bytes: bs58.encode(Buffer.from(name)),
                offset: 101,
              },
            },
          ],
        }
      );

      expect(accts.length).to.equal(1);
      expect(!!accts[0].account.data.toString().match(/new metadata/));
    });

    it("should delete a metadata entry", async () => {
      const deleteIx = await programMetadata.deleteMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        name
      );

      const tx = new Transaction();
      const signers: Keypair[] = [];
      signers.push(privateKeypair);

      tx.add(deleteIx);

      await connection.sendTransaction(tx, signers);

      await timeout(5000);

      const accts = await connection.getProgramAccounts(
        programMetadata.nameServiceKey,
        {
          filters: [
            {
              memcmp: {
                bytes: bs58.encode(Buffer.from(name)),
                offset: 101,
              },
            },
          ],
        }
      );

      expect(accts.length).to.equal(0);
    });

    after(async () => {
      const deleteIx = await programMetadata.deleteMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        name
      );

      const tx = new Transaction();
      const signers: Keypair[] = [];
      signers.push(privateKeypair);
      tx.add(deleteIx);

      try {
        await connection.sendTransaction(tx, signers);
      } catch (error) {
        // we don't care if this fails here
      }

      await timeout(5000);
    });
  });

  describe("retrieve metadata entries", async () => {
    before(async () => {
      const ix1 = await programMetadata.createMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        "Name",
        "My Program"
      );

      const ix2 = await programMetadata.createMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        "Version",
        "1.0.0"
      );

      let tx = new Transaction();
      const signers: Keypair[] = [];
      signers.push(privateKeypair);
      tx.add(ix1);
      tx.add(ix2);

      let res = await connection.sendTransaction(tx, signers);

      await timeout(5000);
    });

    it("should retrieve all metadata entries", async () => {
      const entries = await programMetadata.getMetadataEntries(
        targetProgramKey
      );
      const nameEntry = entries.find((value) => value.name === "Name");
      expect(nameEntry?.value).to.equal("My Program");
      const versionEntry = entries.find((value) => value.name === "Version");
      expect(versionEntry?.value).to.equal("1.0.0");
    });

    after(async () => {
      const deleteIx1 = await programMetadata.deleteMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        "Name"
      );

      const deleteIx2 = await programMetadata.deleteMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        "Version"
      );

      let tx = new Transaction();
      const signers: Keypair[] = [];
      signers.push(privateKeypair);
      tx = new Transaction();
      tx.add(deleteIx1);
      tx.add(deleteIx2);

      const res = await connection.sendTransaction(tx, signers);

      await timeout(5000);
    });
  });
});

describe("ProgramMetadata: VersionedIdl", async () => {
  describe("create, update, delete", async () => {
    const effectiveSlot = 3000;

    const signers: Keypair[] = [privateKeypair];

    before(async () => {
      const idlHash = createHash("sha256").update("some idl", "utf8").digest();

      const ix = await programMetadata.createVersionedIdl(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        effectiveSlot,
        "http://www.test.com",
        idlHash,
        "https://github.com/source"
      );

      const tx = new Transaction();
      tx.add(ix);

      await connection.sendTransaction(tx, signers);

      await timeout(5000);
    });

    it("should create a versioned idl", async () => {
      const classKey = await programMetadata.getClassKey(targetProgramKey);

      const accts = await connection.getProgramAccounts(
        programMetadata.nameServiceKey,
        {
          filters: [
            {
              memcmp: {
                bytes: classKey.toBase58(),
                offset: NAME_SERVICE_CLASS_OFFSET,
              },
            },
            {
              memcmp: {
                bytes: bs58.encode(Buffer.from([1])),
                offset: NAME_SERVICE_ACCOUNT_OFFSET,
              },
            },
          ],
        }
      );

      expect(accts.length).to.equal(1);
    });

    it("should update a versioned idl", async () => {
      const idlHash = createHash("sha256").update("some idl", "utf8").digest();

      const ix = await programMetadata.updateVersionedIdl(
        targetProgramKey,
        privateKeypair.publicKey,
        effectiveSlot,
        "http://www.test2.com",
        idlHash,
        "http://www.github.com/source"
      );

      const tx = new Transaction();
      tx.add(ix);

      await connection.sendTransaction(tx, signers);

      await timeout(5000);

      const classKey = await programMetadata.getClassKey(targetProgramKey);

      const accts = await connection.getProgramAccounts(
        programMetadata.nameServiceKey,
        {
          filters: [
            {
              memcmp: {
                bytes: classKey.toBase58(),
                offset: NAME_SERVICE_CLASS_OFFSET,
              },
            },
            {
              memcmp: {
                bytes: bs58.encode(Buffer.from([1])),
                offset: NAME_SERVICE_ACCOUNT_OFFSET,
              },
            },
          ],
        }
      );

      expect(accts.length).to.equal(1);

      const idl: VersionedIdl = VersionedIdl.decodeUnchecked(
        accts[0].account.data.slice(96)
      );
      expect(idl.idlUrl).to.equal("http://www.test2.com");
    });

    it("should delete a versioned idl", async () => {
      const deleteIx = await programMetadata.deleteMetadataEntry(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        `idl_${effectiveSlot}`
      );

      let tx = new Transaction();
      tx.add(deleteIx);

      await connection.sendTransaction(tx, signers);

      await timeout(5000);

      const classKey = await programMetadata.getClassKey(targetProgramKey);

      const accts = await connection.getProgramAccounts(
        programMetadata.nameServiceKey,
        {
          filters: [
            {
              memcmp: {
                bytes: classKey.toBase58(),
                offset: NAME_SERVICE_CLASS_OFFSET,
              },
            },
            {
              memcmp: {
                bytes: bs58.encode(Buffer.from([1])),
                offset: NAME_SERVICE_ACCOUNT_OFFSET,
              },
            },
          ],
        }
      );

      expect(accts.length).to.equal(0);
    });

    after(async () => {
      try {
        const deleteIx = await programMetadata.deleteMetadataEntry(
          targetProgramKey,
          privateKeypair.publicKey,
          privateKeypair.publicKey,
          `idl_${effectiveSlot}`
        );

        let tx = new Transaction();
        tx.add(deleteIx);

        await connection.sendTransaction(tx, signers);

        await timeout(5000);
      } catch (error) {}
    });
  });

  describe("retrieve IDL entries", async () => {
    const signers: Keypair[] = [];
    signers.push(privateKeypair);

    before(async () => {
      const idlHash = createHash("sha256").update("some idl", "utf8").digest();

      const ix1 = await programMetadata.createVersionedIdl(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        0,
        "http://www.test.com",
        idlHash,
        "https://github.com/source"
      );

      const ix2 = await programMetadata.createVersionedIdl(
        targetProgramKey,
        privateKeypair.publicKey,
        privateKeypair.publicKey,
        3000,
        "http://www.test.com",
        idlHash,
        "https://github.com/source"
      );

      const tx = new Transaction();
      tx.add(ix1).add(ix2);

      await connection.sendTransaction(tx, signers);

      await timeout(5000);
    });

    it("should get the versioned idl accounts", async () => {
      const idls = await programMetadata.getVersionedIdls(targetProgramKey);
      expect(idls.length).to.equal(2);
    });

    it("should get the version appropriate for the slot", async () => {
      let idl = await programMetadata.getVersionedIdlForSlot(
        targetProgramKey,
        3001
      );
      expect(idl?.effectiveSlot.eq(new BN(3000)));

      idl = await programMetadata.getVersionedIdlForSlot(targetProgramKey, 50);
      expect(idl?.effectiveSlot.eq(new BN(0)));
    });

    after(async () => {
      try {
        const deleteIx1 = await programMetadata.deleteVersionedIdl(
          targetProgramKey,
          privateKeypair.publicKey,
          privateKeypair.publicKey,
          0
        );

        const deleteIx2 = await programMetadata.deleteVersionedIdl(
          targetProgramKey,
          privateKeypair.publicKey,
          privateKeypair.publicKey,
          3000
        );

        let tx = new Transaction();
        tx.add(deleteIx1).add(deleteIx2);

        await connection.sendTransaction(tx, signers);

        await timeout(5000);
      } catch (error) {}
    });
  });
});
