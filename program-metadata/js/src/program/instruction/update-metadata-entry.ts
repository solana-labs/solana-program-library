import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class UpdateMetadataEntryInstruction extends Struct {
  instruction = 1;

  constructor(params: { value: string }) {
    super(params);
  }
}

PROGRAM_METADATA_SCHEMA.set(UpdateMetadataEntryInstruction, {
  kind: "struct",
  fields: [
    ["instruction", "u8"],
    ["value", "string"],
  ],
});
