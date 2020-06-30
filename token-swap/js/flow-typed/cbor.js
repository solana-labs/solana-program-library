declare module 'cbor' {
  declare module.exports: {
    decode(input: Buffer): Object;
    encode(input: any): Buffer;
  };
}
