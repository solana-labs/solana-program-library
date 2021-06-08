import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class CreateMetadataEntryInstruction extends Struct {
  instruction = 0;

  constructor(params: { name: string; value: string; hashedName: Buffer }) {
    super(params);
  }
}

PROGRAM_METADATA_SCHEMA.set(CreateMetadataEntryInstruction, {
  kind: "struct",
  fields: [
    ["instruction", "u8"],
    ["name", "string"],
    ["value", "string"],
    ["hashedName", [32]],
  ],
});
