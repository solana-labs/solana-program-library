import {
  Connection,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";
import BN from "bn.js";
import bs58 from "bs58";
import { createHash } from "crypto";
import { MetadataEntry } from "./accounts/metadata-entry";
import { VersionedIdl } from "./accounts/versioned-idl";
import {
  createMetadataEntryIx,
  createVersionedIdlIx,
  deleteMetadataEntryIx,
  updateMetadataEntryIx,
  updateVersionedIdlIx,
} from "./instruction";

export const NS_HASH_PREFIX = "SPL Name Service";
export const NS_PROGRAM_ID = new PublicKey(
  "2eD37nsnRfY7QdymU6GXrkZ7rUhpL6Y29e8K8dhisN7G"
);

export const METADATA_PREFIX = "program_metadata";
export const PROGRAM_METADATA_ID = new PublicKey(
  "6cQ31NiNjrTTvjFbXiDUxo2ao29jQrGpV2JkN1Ztm2Gy"
);

export const NAME_SERVICE_CLASS_OFFSET = 64;
export const NAME_SERVICE_ACCOUNT_OFFSET = 96;

export interface ProgramMetadataConfig {
  programMetadataKey?: PublicKey;
  nameServiceKey?: PublicKey;
}

export class ProgramMetadata {
  programMetadataKey = PROGRAM_METADATA_ID;

  nameServiceKey = NS_PROGRAM_ID;

  constructor(private connection: Connection, config?: ProgramMetadataConfig) {
    if (config?.programMetadataKey) {
      this.programMetadataKey = config.programMetadataKey;
    }

    if (config?.nameServiceKey) {
      this.nameServiceKey = config.nameServiceKey;
    }
  }

  async getVersionedIdls(programId: PublicKey) {
    const classKey = await this.getClassKey(programId);

    let accounts = await this.connection.getProgramAccounts(
      this.nameServiceKey,
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

    return accounts
      .map((account) => {
        try {
          return VersionedIdl.decodeUnchecked(account.account.data.slice(96));
        } catch (error) {
          console.error(error);
        }
      })
      .filter((i) => i instanceof VersionedIdl);
  }

  async getVersionedIdlForSlot(
    programId: PublicKey,
    slot: number
  ): Promise<VersionedIdl | undefined> {
    const idls = await this.getVersionedIdls(programId);
    const sorted: VersionedIdl[] = idls.sort(
      (idlA: VersionedIdl, idlB: VersionedIdl) => {
        return idlA.effectiveSlot.gt(idlB.effectiveSlot) ? 0 : -1;
      }
    );
    let result;
    for (let idl of sorted) {
      if (idl.effectiveSlot.lte(new BN(slot))) {
        result = idl;
      }
    }
    return result;
  }

  async getMetadataEntries(programId: PublicKey): Promise<MetadataEntry[]> {
    const classKey = await this.getClassKey(programId);

    let accounts = await this.connection.getProgramAccounts(
      this.nameServiceKey,
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
              bytes: bs58.encode(Buffer.from([0])),
              offset: NAME_SERVICE_ACCOUNT_OFFSET,
            },
          },
        ],
      }
    );

    return accounts
      .map((account) => {
        try {
          return MetadataEntry.decodeUnchecked(account.account.data.slice(96));
        } catch (error) {
          console.error(error);
        }
      })
      .filter((i) => i instanceof MetadataEntry);
  }

  async createMetadataEntry(
    targetProgramId: PublicKey,
    targetProgramAuthorityKey: PublicKey,
    payerKey: PublicKey,
    name: string,
    value: string
  ): Promise<TransactionInstruction> {
    const hashedName = this.getHashedName(name);
    const classKey = await this.getClassKey(targetProgramId);
    const nameKey = await this.getNameKey(hashedName, classKey);

    const targetProgramAcct = await this.connection.getAccountInfo(
      targetProgramId
    );

    if (!targetProgramAcct) {
      throw new Error("Program not found");
    }

    const targetProgramDataKey = new PublicKey(targetProgramAcct.data.slice(3));

    const ix = createMetadataEntryIx(
      this.programMetadataKey,
      classKey,
      nameKey,
      targetProgramId,
      targetProgramDataKey,
      targetProgramAuthorityKey,
      payerKey,
      SystemProgram.programId,
      SYSVAR_RENT_PUBKEY,
      this.nameServiceKey,
      name,
      value,
      hashedName
    );

    return ix;
  }

  async updateMetadataEntry(
    targetProgramId: PublicKey,
    targetProgramAuthorityKey: PublicKey,
    name: string,
    value: string
  ): Promise<TransactionInstruction> {
    const hashedName = this.getHashedName(name);
    const classKey = await this.getClassKey(targetProgramId);
    const nameKey = await this.getNameKey(hashedName, classKey);

    const targetProgramAcct = await this.connection.getAccountInfo(
      targetProgramId
    );

    if (!targetProgramAcct) {
      throw new Error("Program not found");
    }

    const targetProgramDataKey = new PublicKey(targetProgramAcct.data.slice(3));

    const ix = updateMetadataEntryIx(
      this.programMetadataKey,
      classKey,
      nameKey,
      targetProgramId,
      targetProgramDataKey,
      targetProgramAuthorityKey,
      this.nameServiceKey,
      value
    );

    return ix;
  }

  async deleteMetadataEntry(
    targetProgramId: PublicKey,
    targetProgramAuthorityKey: PublicKey,
    refundKey: PublicKey,
    name: string
  ): Promise<TransactionInstruction> {
    const hashedName = this.getHashedName(name);
    const classKey = await this.getClassKey(targetProgramId);
    const nameKey = await this.getNameKey(hashedName, classKey);

    const targetProgramAcct = await this.connection.getAccountInfo(
      targetProgramId
    );

    if (!targetProgramAcct) {
      throw new Error("Program not found");
    }

    const targetProgramDataKey = new PublicKey(targetProgramAcct.data.slice(3));

    const ix = deleteMetadataEntryIx(
      this.programMetadataKey,
      classKey,
      nameKey,
      targetProgramId,
      targetProgramDataKey,
      targetProgramAuthorityKey,
      refundKey,
      this.nameServiceKey
    );

    return ix;
  }

  async deleteVersionedIdl(
    targetProgramId: PublicKey,
    targetProgramAuthorityKey: PublicKey,
    refundKey: PublicKey,
    effectiveSlot: number
  ): Promise<TransactionInstruction> {
    return this.deleteMetadataEntry(
      targetProgramId,
      targetProgramAuthorityKey,
      refundKey,
      `idl_${effectiveSlot}`
    );
  }

  async createVersionedIdl(
    targetProgramId: PublicKey,
    targetProgramAuthorityKey: PublicKey,
    payerKey: PublicKey,
    effectiveSlot: number,
    idlUrl: string,
    idlHash: Buffer,
    sourceUrl: string
  ): Promise<TransactionInstruction> {
    const name = `idl_${effectiveSlot}`;
    const hashedName = this.getHashedName(name);
    const classKey = await this.getClassKey(targetProgramId);
    const nameKey = await this.getNameKey(hashedName, classKey);

    const targetProgramAcct = await this.connection.getAccountInfo(
      targetProgramId
    );

    if (!targetProgramAcct) {
      throw new Error("Program not found");
    }

    const targetProgramDataKey = new PublicKey(targetProgramAcct.data.slice(3));

    const ix = createVersionedIdlIx(
      this.programMetadataKey,
      classKey,
      nameKey,
      targetProgramId,
      targetProgramDataKey,
      targetProgramAuthorityKey,
      payerKey,
      SystemProgram.programId,
      SYSVAR_RENT_PUBKEY,
      this.nameServiceKey,
      effectiveSlot,
      idlUrl,
      idlHash,
      sourceUrl,
      hashedName
    );

    return ix;
  }

  async updateVersionedIdl(
    targetProgramId: PublicKey,
    targetProgramAuthorityKey: PublicKey,
    effectiveSlot: number,
    idlUrl: string,
    idlHash: Buffer,
    sourceUrl: string
  ): Promise<TransactionInstruction> {
    const hashedName = this.getHashedName(`idl_${effectiveSlot}`);
    const classKey = await this.getClassKey(targetProgramId);
    const nameKey = await this.getNameKey(hashedName, classKey);

    const targetProgramAcct = await this.connection.getAccountInfo(
      targetProgramId
    );

    if (!targetProgramAcct) {
      throw new Error("Program not found");
    }

    const targetProgramDataKey = new PublicKey(targetProgramAcct.data.slice(3));

    const ix = updateVersionedIdlIx(
      this.programMetadataKey,
      classKey,
      nameKey,
      targetProgramId,
      targetProgramDataKey,
      targetProgramAuthorityKey,
      this.nameServiceKey,
      idlUrl,
      idlHash,
      sourceUrl
    );

    return ix;
  }

  getHashedName(name) {
    let input = NS_HASH_PREFIX + name;
    let buffer = createHash("sha256").update(input, "utf8").digest();
    return buffer;
  }

  async getClassKey(targetProgramId: PublicKey) {
    const [classKey] = await PublicKey.findProgramAddress(
      [Buffer.from(METADATA_PREFIX), targetProgramId.toBuffer()],
      this.programMetadataKey
    );

    return classKey;
  }

  async getNameKey(hashedName: Buffer, classKey: PublicKey) {
    const [nameKey] = await PublicKey.findProgramAddress(
      [hashedName, classKey.toBuffer(), Buffer.alloc(32)],
      this.nameServiceKey
    );

    return nameKey;
  }
}
