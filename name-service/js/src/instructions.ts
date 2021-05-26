import { PublicKey, TransactionInstruction } from '@solana/web3.js';

import { Numberu32, Numberu64 } from './utils';

export function createInstruction(
  nameProgramId: PublicKey,
  systemProgramId: PublicKey,
  nameKey: PublicKey,
  nameOwnerKey: PublicKey,
  payerKey: PublicKey,
  hashed_name: Buffer,
  lamports: Numberu64,
  space: Numberu32,
  nameClassKey?: PublicKey,
  nameParent?: PublicKey,
  nameParentOwner?: PublicKey
): TransactionInstruction {
  const buffers = [
    Buffer.from(Int8Array.from([0])),
    new Numberu32(hashed_name.length).toBuffer(),
    hashed_name,
    lamports.toBuffer(),
    space.toBuffer(),
  ];

  const data = Buffer.concat(buffers);

  const keys = [
    {
      pubkey: systemProgramId,
      isSigner: false,
      isWritable: false,
    },
    {
      pubkey: payerKey,
      isSigner: true,
      isWritable: true,
    },
    {
      pubkey: nameKey,
      isSigner: false,
      isWritable: true,
    },
    {
      pubkey: nameOwnerKey,
      isSigner: false,
      isWritable: false,
    },
  ];

  if (nameClassKey) {
    keys.push({
      pubkey: nameClassKey,
      isSigner: true,
      isWritable: false,
    });
  } else {
    keys.push({
      pubkey: new PublicKey(Buffer.alloc(32)),
      isSigner: false,
      isWritable: false,
    });
  }
  if (nameParent) {
    keys.push({
      pubkey: nameParent,
      isSigner: false,
      isWritable: false,
    });
  } else {
    keys.push({
      pubkey: new PublicKey(Buffer.alloc(32)),
      isSigner: false,
      isWritable: false,
    });
  }
  if (nameParentOwner) {
    keys.push({
      pubkey: nameParentOwner,
      isSigner: true,
      isWritable: false,
    });
  }

  return new TransactionInstruction({
    keys,
    programId: nameProgramId,
    data,
  });
}

export function updateInstruction(
  nameProgramId: PublicKey,
  nameAccountKey: PublicKey,
  offset: Numberu32,
  input_data: Buffer,
  nameUpdateSigner: PublicKey
): TransactionInstruction {
  const buffers = [
    Buffer.from(Int8Array.from([1])),
    offset.toBuffer(),
    new Numberu32(input_data.length).toBuffer(),
    input_data,
  ];

  const data = Buffer.concat(buffers);
  const keys = [
    {
      pubkey: nameAccountKey,
      isSigner: false,
      isWritable: true,
    },
    {
      pubkey: nameUpdateSigner,
      isSigner: true,
      isWritable: false,
    },
  ];

  return new TransactionInstruction({
    keys,
    programId: nameProgramId,
    data,
  });
}

export function transferInstruction(
  nameProgramId: PublicKey,
  nameAccountKey: PublicKey,
  newOwnerKey: PublicKey,
  currentNameOwnerKey: PublicKey,
  nameClassKey?: PublicKey
): TransactionInstruction {
  const buffers = [Buffer.from(Int8Array.from([2])), newOwnerKey.toBuffer()];

  const data = Buffer.concat(buffers);

  const keys = [
    {
      pubkey: nameAccountKey,
      isSigner: false,
      isWritable: true,
    },
    {
      pubkey: currentNameOwnerKey,
      isSigner: true,
      isWritable: false,
    },
  ];

  if (nameClassKey) {
    keys.push({
      pubkey: nameClassKey,
      isSigner: true,
      isWritable: false,
    });
  }

  return new TransactionInstruction({
    keys,
    programId: nameProgramId,
    data,
  });
}

export function deleteInstruction(
  nameProgramId: PublicKey,
  nameAccountKey: PublicKey,
  refundTargetKey: PublicKey,
  nameOwnerKey: PublicKey
): TransactionInstruction {
  const buffers = [Buffer.from(Int8Array.from([3]))];

  const data = Buffer.concat(buffers);
  const keys = [
    {
      pubkey: nameAccountKey,
      isSigner: false,
      isWritable: true,
    },
    {
      pubkey: nameOwnerKey,
      isSigner: true,
      isWritable: false,
    },
    {
      pubkey: refundTargetKey,
      isSigner: false,
      isWritable: true,
    },
  ];

  return new TransactionInstruction({
    keys,
    programId: nameProgramId,
    data,
  });
}
