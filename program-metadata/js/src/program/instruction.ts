import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { UpdateMetadataEntryInstruction } from "./instruction/update-metadata-entry";
import { CreateVersionedIdlInstruction } from "./instruction/create-versioned-idl";
import { UpdateVersionedIdlInstruction } from "./instruction/update-versioned-idl";
import { CreateMetadataEntryInstruction } from "./instruction/create-metadata-entry";
import { DeleteMetadataEntry } from "./instruction/delete-metadata-entry";

export function createMetadataEntryIx(
  programId: PublicKey,
  classKey: PublicKey,
  nameKey: PublicKey,
  targetProgramKey: PublicKey,
  targetProgramDataKey: PublicKey,
  targetProgramAuthorityKey: PublicKey,
  payerKey: PublicKey,
  systemProgramId: PublicKey,
  rentKey: PublicKey,
  nameServiceKey: PublicKey,
  name: string,
  value: string,
  hashedName: Buffer
): TransactionInstruction {
  const ixDataObject = new CreateMetadataEntryInstruction(
    name,
    value,
    hashedName
  );

  const ixData = ixDataObject.encode();

  const ix = new TransactionInstruction({
    programId: programId,
    keys: [
      { pubkey: classKey, isSigner: false, isWritable: false },
      { pubkey: nameKey, isSigner: false, isWritable: true },
      { pubkey: targetProgramKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
      { pubkey: payerKey, isSigner: true, isWritable: true },
      { pubkey: systemProgramId, isSigner: false, isWritable: false },
      { pubkey: rentKey, isSigner: false, isWritable: false },
      { pubkey: nameServiceKey, isSigner: false, isWritable: false },
    ],
    data: ixData,
  });

  return ix;
}

export function updateMetadataEntryIx(
  programId: PublicKey,
  classKey: PublicKey,
  nameKey: PublicKey,
  targetProgramKey: PublicKey,
  targetProgramDataKey: PublicKey,
  targetProgramAuthorityKey: PublicKey,
  nameServiceKey: PublicKey,
  value: string
): TransactionInstruction {
  const ixDataObject = new UpdateMetadataEntryInstruction(value);

  const ixData = ixDataObject.encode();

  const ix = new TransactionInstruction({
    programId: programId,
    keys: [
      { pubkey: classKey, isSigner: false, isWritable: false },
      { pubkey: nameKey, isSigner: false, isWritable: true },
      { pubkey: targetProgramKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
      { pubkey: nameServiceKey, isSigner: false, isWritable: false },
    ],
    data: ixData,
  });

  return ix;
}

export function deleteMetadataEntryIx(
  programId: PublicKey,
  classKey: PublicKey,
  nameKey: PublicKey,
  targetProgramKey: PublicKey,
  targetProgramDataKey: PublicKey,
  targetProgramAuthorityKey: PublicKey,
  refundKey: PublicKey,
  nameServiceKey: PublicKey
) {
  const ixDataObject = new DeleteMetadataEntry();

  const ixData = ixDataObject.encode();

  const ix = new TransactionInstruction({
    programId: programId,
    keys: [
      { pubkey: classKey, isSigner: false, isWritable: false },
      { pubkey: nameKey, isSigner: false, isWritable: true },
      { pubkey: targetProgramKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
      { pubkey: refundKey, isSigner: false, isWritable: false },
      { pubkey: nameServiceKey, isSigner: false, isWritable: false },
    ],
    data: ixData,
  });

  return ix;
}

export function createVersionedIdlIx(
  programId: PublicKey,
  classKey: PublicKey,
  nameKey: PublicKey,
  targetProgramKey: PublicKey,
  targetProgramDataKey: PublicKey,
  targetProgramAuthorityKey: PublicKey,
  payerKey: PublicKey,
  systemProgramId: PublicKey,
  rentKey: PublicKey,
  nameServiceKey: PublicKey,
  effectiveSlot: number,
  idlUrl: string,
  idlHash: Buffer,
  sourceUrl: string,
  hashedName: Buffer
) {
  const ixDataObject = new CreateVersionedIdlInstruction(
    effectiveSlot,
    idlUrl,
    idlHash,
    sourceUrl,
    hashedName
  );

  const ixData = ixDataObject.encode();

  const ix = new TransactionInstruction({
    programId: programId,
    keys: [
      { pubkey: classKey, isSigner: false, isWritable: false },
      { pubkey: nameKey, isSigner: false, isWritable: true },
      { pubkey: targetProgramKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
      { pubkey: payerKey, isSigner: true, isWritable: true },
      { pubkey: systemProgramId, isSigner: false, isWritable: false },
      { pubkey: rentKey, isSigner: false, isWritable: false },
      { pubkey: nameServiceKey, isSigner: false, isWritable: false },
    ],
    data: ixData,
  });

  return ix;
}

export function updateVersionedIdlIx(
  programId: PublicKey,
  classKey: PublicKey,
  nameKey: PublicKey,
  targetProgramKey: PublicKey,
  targetProgramDataKey: PublicKey,
  targetProgramAuthorityKey: PublicKey,
  nameServiceKey: PublicKey,
  idlUrl: string,
  idlHash: Buffer,
  sourceUrl: string
): TransactionInstruction {
  const ixDataObject = new UpdateVersionedIdlInstruction(
    idlUrl,
    idlHash,
    sourceUrl
  );

  const ixData = ixDataObject.encode();

  const ix = new TransactionInstruction({
    programId: programId,
    keys: [
      { pubkey: classKey, isSigner: false, isWritable: false },
      { pubkey: nameKey, isSigner: false, isWritable: true },
      { pubkey: targetProgramKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
      { pubkey: nameServiceKey, isSigner: false, isWritable: false },
    ],
    data: ixData,
  });

  return ix;
}
