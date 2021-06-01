import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class UpdateVersionedIdlInstruction extends Struct {
  instruction = 4;

  constructor(
    public idlUrl: string,
    public idlHash: Buffer,
    public sourceUrl: string
  ) {
    super();
  }
}

PROGRAM_METADATA_SCHEMA.set(UpdateVersionedIdlInstruction, {
  kind: "struct",
  fields: [
    ["instruction", "u8"],
    ["idlUrl", "string"],
    ["idlHash", [32]],
    ["sourceUrl", "string"],
  ],
});
