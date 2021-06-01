import { SerializationMethod } from "../../idl/idl-coder";
import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class CreateMetadataEntryInstruction extends Struct {
  instruction = 0;

  constructor(
    public name: string,
    public value: string,
    public hashedName: Buffer
  ) {
    super();
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
