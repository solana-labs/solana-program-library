import { Buffer } from "buffer";
import { serialize, deserializeUnchecked, deserialize } from "borsh";

// Class wrapping a plain object
export class Struct {
  constructor(properties: any) {
    Object.assign(this, properties);
  }

  encode(): Buffer {
    return Buffer.from(serialize(PROGRAM_METADATA_SCHEMA, this));
  }

  static decode(data: Buffer): any {
    return deserialize(PROGRAM_METADATA_SCHEMA, this, data);
  }

  static decodeUnchecked(data: Buffer): any {
    return deserializeUnchecked(PROGRAM_METADATA_SCHEMA, this, data);
  }
}

export const PROGRAM_METADATA_SCHEMA: Map<Function, any> = new Map();
