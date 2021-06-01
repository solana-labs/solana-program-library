import { SerializationMethod } from "../../idl/idl-coder";
import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class CreateVersionedIdlInstruction extends Struct {
  instruction = 3;
  serialization;

  constructor(
    public effectiveSlot: number,
    public idlUrl: string,
    public idlHash: Buffer,
    public sourceUrl: string,
    serialization: SerializationMethod,
    public customLayoutUrl: null | string,
    public hashedName: Buffer
  ) {
    super();
    this.serialization = [serialization];
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
    ["serialization", [1]],
    [
      "customLayoutUrl",
      {
        kind: "option",
        type: "string",
      },
    ],
    ["hashedName", [32]],
  ],
});
