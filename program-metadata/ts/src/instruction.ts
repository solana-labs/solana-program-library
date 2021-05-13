import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { serialize } from "borsh";
import { Numberu32 } from "./util";

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
  const encodedName = Buffer.from(name);
  const encodedData = Buffer.from(value);

  let buffers = [
    Buffer.from(Int8Array.from([0])),
    new Numberu32(encodedName.length).toBuffer(),
    encodedName,
    new Numberu32(encodedData.length).toBuffer(),
    encodedData,
    new Numberu32(hashedName.length).toBuffer(),
    hashedName
  ];

  const ixData = Buffer.concat(buffers);

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
      { pubkey: nameServiceKey, isSigner: false, isWritable: false }
    ],
    data: Buffer.from(ixData)
  })

  return ix;
}

class UpdateMetadataEntryInstruction {
  instruction = [1];
  constructor(public value: string) { }
}

const UpdateMetadataEntrySchema = new Map([[UpdateMetadataEntryInstruction, {
  kind: 'struct',
  fields: [
    ['instruction', [1]],
    ['value', 'string']
  ]
}]]);

export function updateMetadataEntryIx(
  programId: PublicKey,
  classKey: PublicKey,
  nameKey: PublicKey,
  targetProgramKey: PublicKey,
  targetProgramDataKey: PublicKey,
  targetProgramAuthorityKey: PublicKey,
  nameServiceKey: PublicKey,
  value: string,
): TransactionInstruction {
  const ixDataObject = new UpdateMetadataEntryInstruction(
    value
  );

  const ixData = serialize(UpdateMetadataEntrySchema, ixDataObject);

  const ix = new TransactionInstruction({
    programId: programId,
    keys: [
      { pubkey: classKey, isSigner: false, isWritable: false },
      { pubkey: nameKey, isSigner: false, isWritable: true },
      { pubkey: targetProgramKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
      { pubkey: nameServiceKey, isSigner: false, isWritable: false }
    ],
    data: Buffer.from(ixData)
  })

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
  nameServiceKey: PublicKey,
) {

  const ixData = Buffer.from(Int8Array.from([2]));
  const ix = new TransactionInstruction({
    programId: programId,
    keys: [
      { pubkey: classKey, isSigner: false, isWritable: false },
      { pubkey: nameKey, isSigner: false, isWritable: true },
      { pubkey: targetProgramKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
      { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
      { pubkey: refundKey, isSigner: false, isWritable: false },
      { pubkey: nameServiceKey, isSigner: false, isWritable: false }
    ],
    data: ixData
  });

  return ix;
}