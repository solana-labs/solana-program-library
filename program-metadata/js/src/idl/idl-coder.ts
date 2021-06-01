import { TransactionInstruction } from "@solana/web3.js";
import { Coder, DecodedInstruction } from "./coder";
import { Borsh } from "./coders/borsh";
import { Idl } from "./idl";

export enum SerializationMethod {
  Bincode = 0,
  Borsh = 1,
  Anchor = 2,
}

export const CODER_MAP = new Map<SerializationMethod, new (idl: Idl) => Coder>([
  [SerializationMethod.Borsh, Borsh],
]);

export class IdlCoder {
  private coder: Coder;

  constructor(
    private idl: Idl,
    private serializationMethod: SerializationMethod
  ) {
    const coder = CODER_MAP.get(serializationMethod);
    if (!coder) {
      throw new Error("Serialization method not supported");
    }
    this.coder = new coder(idl);
  }

  decodeInstruction(instruction: TransactionInstruction): DecodedInstruction {
    return this.coder.decodeInstruction(instruction);
  }

  decodeAccount(account) {}
}
