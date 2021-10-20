import {PublicKey} from '@solana/web3.js';
import * as BufferLayout from 'buffer-layout';

/**
 * Layout for a public key
 */
export const publicKey = (
  property: string = 'publicKey',
): BufferLayout.Layout<PublicKey> => {
  return BufferLayout.blob(
    32,
    property,
  ) as unknown as BufferLayout.Layout<PublicKey>;
};

/**
 * Layout for a 64bit unsigned value
 */
export const uint64 = (
  property: string = 'uint64',
): BufferLayout.Layout<Buffer> => {
  return BufferLayout.blob(8, property);
};

/**
 * Layout for a Rust String type
 */
type RawRustStringLayout = {
  length: number;
  lengthPadding: number;
  chars: Buffer;
};
type RustStringDecode = BufferLayout.Layout<RawRustStringLayout>['decode'];
type RustStringEncode = BufferLayout.Layout<
  Pick<RawRustStringLayout, 'chars'>
>['encode'];

export const rustString = (
  property: string = 'string',
): BufferLayout.Layout<string> => {
  const rsl = BufferLayout.struct<string>(
    [
      BufferLayout.u32('length'),
      BufferLayout.u32('lengthPadding'),
      BufferLayout.blob(BufferLayout.offset(BufferLayout.u32(), -8), 'chars'),
    ],
    property,
  );
  const _decode = rsl.decode.bind(rsl) as unknown as RustStringDecode;
  const _encode = rsl.encode.bind(rsl) as unknown as RustStringEncode;

  rsl.decode = (buffer: Buffer, offset: number) => {
    const data = _decode(buffer, offset);
    return data.chars.toString('utf8');
  };

  rsl.encode = (str: string, buffer: Buffer, offset: number) => {
    const data = {
      chars: Buffer.from(str, 'utf8'),
    };
    return _encode(data, buffer, offset);
  };

  return rsl;
};
