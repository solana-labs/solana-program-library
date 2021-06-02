import { TransactionInstruction } from "@solana/web3.js";
import { Coder, DecodedInstruction } from "./coder";
import { Anchor } from "./coders/anchor";
import { Borsh } from "./coders/borsh";
import { Idl, SerializationMethod } from "./idl";

const DEFAULT_SERIALIZATION_METHOD = SerializationMethod.Anchor;

export const CODER_MAP = new Map<SerializationMethod, new (idl: Idl) => Coder>([
  [SerializationMethod.Anchor, Anchor],
  [SerializationMethod.Borsh, Borsh],
]);

export class IdlCoder {
  private coder: Coder;

  constructor(private idl: Idl) {
    const serializationMethod =
      idl.serializationMethod || DEFAULT_SERIALIZATION_METHOD;

    const coder = CODER_MAP.get(serializationMethod);
    if (!coder) {
      throw new Error("Serialization method not supported");
    }
    this.coder = new coder(idl);
  }

  decodeInstruction(instruction: TransactionInstruction): DecodedInstruction {
    return this.coder.decodeInstruction(instruction);
  }
}
