import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class MetadataEntry extends Struct {
  accountType = 0;
  name!: string;
  value!: string;

  constructor(params: { name: string; value: string }) {
    super(params);
  }
}

PROGRAM_METADATA_SCHEMA.set(MetadataEntry, {
  kind: "struct",
  fields: [
    ["accountType", "u8"],
    ["name", "string"],
    ["value", "string"],
  ],
});
