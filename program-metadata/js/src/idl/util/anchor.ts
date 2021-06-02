import { snakeCase } from "snake-case";
import { sha256 } from "js-sha256";
import { fieldLayout } from "./borsh";
import { IdlField, IdlInstruction, IdlTypeDef } from "../idl";
import * as borsh from "@project-serum/borsh";

/**
 * Namespace for state method function signatures.
 */
export const SIGHASH_STATE_NAMESPACE = "state";
/**
 * Namespace for global instruction function signatures (i.e. functions
 * that aren't namespaced by the state or any of its trait implementations).
 */
export const SIGHASH_GLOBAL_NAMESPACE = "global";

// Not technically sighash, since we don't include the arguments, as Rust
// doesn't allow function overloading.
export function sighash(nameSpace: string, ixName: string): Buffer {
  let name = snakeCase(ixName);
  let preimage = `${nameSpace}:${name}`;
  return Buffer.from(sha256.digest(preimage)).slice(0, 8);
}

export function buildInstructionCoder(
  idlInstruction: IdlInstruction,
  types: IdlTypeDef[]
) {
  const fieldLayouts = idlInstruction.args.map((arg: IdlField) =>
    fieldLayout(arg, types)
  );
  return borsh.struct(fieldLayouts);
}
