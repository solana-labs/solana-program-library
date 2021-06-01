import { SerializationMethod } from "../../idl/idl-coder";
import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class UpdateVersionedIdlInstruction extends Struct {
  instruction = 4;
  serialization;

  constructor(
    public idlUrl: string,
    public idlHash: Buffer,
    public sourceUrl: string,
    serialization: SerializationMethod,
    public customLayoutUrl: null | string
  ) {
    super();
    this.serialization = [serialization];
  }
}

PROGRAM_METADATA_SCHEMA.set(UpdateVersionedIdlInstruction, {
  kind: "struct",
  fields: [
    ["instruction", "u8"],
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
  ],
});
