import { PublicKey } from '@solana/web3.js';
import * as BufferLayout from '@solana/buffer-layout';
import { Buffer } from 'buffer';

// FIXME replace with real id when we have one
export const SINGLE_POOL_PROGRAM_ID = new PublicKey('3cqnsMsT6LE96pxv7GR4di5rLqHDZZbR3FbeSUeRLFqY');

export function defaultDepositAccountSeed(poolAddress: PublicKey): string {
  return 'svsp' + poolAddress.toString().slice(0, 28);
}

export type InstructionType = {
  /** The Instruction index (from solana upstream program) */
  index: number;
  /** The BufferLayout to use to build data */
  layout: BufferLayout.Layout<any>;
};

export function encodeData(type: InstructionType, fields?: any): Buffer {
  const allocLength = type.layout.span;
  const data = Buffer.alloc(allocLength);
  const layoutFields = Object.assign({ instruction: type.index }, fields);
  type.layout.encode(layoutFields, data);

  return data;
}

export function decodeData(type: InstructionType, buffer: Buffer): any {
  let data;
  try {
    data = type.layout.decode(buffer);
  } catch (err) {
    throw new Error('invalid instruction; ' + err);
  }

  if (data.instruction !== type.index) {
    throw new Error(
      `invalid instruction; instruction index mismatch ${data.instruction} != ${type.index}`,
    );
  }

  return data;
}

// UpdateTokenMetadata is omitted here because its size is runtime-dependent
type SinglePoolInstructionType =
  | 'InitializePool'
  | 'DepositStake'
  | 'WithdrawStake'
  | 'CreateTokenMetadata';

export const SINGLE_POOL_INSTRUCTION_LAYOUTS: {
  [type in SinglePoolInstructionType]: InstructionType;
} = Object.freeze({
  InitializePool: {
    index: 0,
    layout: BufferLayout.struct<any>([BufferLayout.u8('instruction')]),
  },
  DepositStake: {
    index: 1,
    layout: BufferLayout.struct<any>([BufferLayout.u8('instruction')]),
  },
  WithdrawStake: {
    index: 2,
    layout: BufferLayout.struct<any>([
      BufferLayout.u8('instruction'),
      BufferLayout.seq(BufferLayout.u8(), 32, 'userStakeAuthority'),
      BufferLayout.ns64('tokenAmount'),
    ]),
  },
  CreateTokenMetadata: {
    index: 3,
    layout: BufferLayout.struct<any>([BufferLayout.u8('instruction')]),
  },
});

export function updateTokenMetadataLayout(
  nameLength: number,
  symbolLength: number,
  uriLength: number,
) {
  if (nameLength > 32) {
    throw 'maximum token name length is 32 characters';
  }

  if (symbolLength > 10) {
    throw 'maximum token symbol length is 10 characters';
  }

  if (uriLength > 200) {
    throw 'maximum token uri length is 200 characters';
  }

  return {
    index: 4,
    layout: BufferLayout.struct<any>([
      BufferLayout.u8('instruction'),
      BufferLayout.u32('tokenNameLen'),
      BufferLayout.blob(nameLength, 'tokenName'),
      BufferLayout.u32('tokenSymbolLen'),
      BufferLayout.blob(symbolLength, 'tokenSymbol'),
      BufferLayout.u32('tokenUriLen'),
      BufferLayout.blob(uriLength, 'tokenUri'),
    ]),
  };
}
