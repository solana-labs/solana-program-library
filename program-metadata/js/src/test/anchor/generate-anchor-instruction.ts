import { InstructionCoder } from "@project-serum/anchor";
import { Idl } from "../../idl/idl";

const idl: Idl = require("../test-idl-anchor.json");

export function generateAnchorInstruction(name: string, ix: any) {
  const coder = new InstructionCoder(idl);
  return coder.encode(name, ix);
}
