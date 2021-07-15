import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class CreateVersionedIdlInstruction extends Struct {
  instruction = 3;

  constructor(params: {
    effectiveSlot: number;
    idlUrl: string;
    idlHash: Buffer;
    sourceUrl: string;
    hashedName: Buffer;
  }) {
    super(params);
  }
}

PROGRAM_METADATA_SCHEMA.set(CreateVersionedIdlInstruction, {
  kind: "struct",
  fields: [
    ["instruction", "u8"],
    ["effectiveSlot", "u64"],
    ["idlUrl", "string"],
    ["idlHash", [32]],
    ["sourceUrl", "string"],
    ["hashedName", [32]],
  ],
});
