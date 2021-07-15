import BN from "bn.js";
import { PROGRAM_METADATA_SCHEMA, Struct } from "../util/borsh-struct";

export class VersionedIdl extends Struct {
  accountType = 1;
  effectiveSlot!: BN;
  idlUrl!: string;
  idlHash!: Buffer;
  sourceURL!: string;

  constructor(params: {
    effectiveSlot: number;
    idlUrl: string;
    idlHash: Buffer;
    sourceUrl: string;
  }) {
    super(params);
  }
}

PROGRAM_METADATA_SCHEMA.set(VersionedIdl, {
  kind: "struct",
  fields: [
    ["accountType", "u8"],
    ["effectiveSlot", "u64"],
    ["idlUrl", "string"],
    ["idlHash", [32]],
    ["sourceUrl", "string"],
  ],
});
