import { Idl } from "@project-serum/anchor";
import { IdlAccountItem, IdlType } from "./idl";
import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { IdlField } from "@project-serum/anchor/dist/idl";

export abstract class Coder {
  constructor(protected idl: Idl) {}
  abstract decodeInstruction(
    instruction: TransactionInstruction
  ): DecodedInstruction;
}

export interface DecodedInstruction {
  name: string;
  formattedName: string;
  programId: PublicKey;
  accounts: DecodedAccount[];
  args: any[];
}

export type DecodedAccount =
  | ({
      formattedName: string;
      pubkey: PublicKey;
    } & IdlAccountItem)
  | AccountError;

export type AccountError = {
  message: string;
};

export type DecodedArgument =
  | ({
      formattedName: string;
      value: any;
    } & IdlField)
  | FieldError;

export type FieldError = {
  message: string;
};
