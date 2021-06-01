import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class DeleteMetadataEntry extends Struct {
  instruction = 2;
}

PROGRAM_METADATA_SCHEMA.set(DeleteMetadataEntry, {
  kind: "struct",
  fields: [["instruction", "u8"]],
});
