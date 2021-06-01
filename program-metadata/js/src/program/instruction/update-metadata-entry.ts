import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class UpdateMetadataEntryInstruction extends Struct {
  instruction = 1;
  constructor(public value: string) {
    super();
  }
}

PROGRAM_METADATA_SCHEMA.set(UpdateMetadataEntryInstruction, {
  kind: "struct",
  fields: [
    ["instruction", "u8"],
    ["value", "string"],
  ],
});
