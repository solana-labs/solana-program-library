import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class UpdateVersionedIdlInstruction extends Struct {
  instruction = 4;

  constructor(params: { idlUrl: string; idlHash: Buffer; sourceUrl: string }) {
    super(params);
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
