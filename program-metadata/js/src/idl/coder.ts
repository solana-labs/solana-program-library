import { Idl, IdlAccountItem, IdlInstruction } from "./idl";
import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { IdlField } from "@project-serum/anchor/dist/idl";
import { startCase } from "../program/util/helpers";

export abstract class Coder {
  constructor(protected idl: Idl) {}

  abstract decodeInstruction(
    instruction: TransactionInstruction
  ): DecodedInstruction;

  public getFormattedName() {
    return startCase(this.idl.name);
  }

  protected getInstruction(index) {
    if (index > this.idl.instructions.length) {
      throw new Error(`Instruction at index ${index} not found`);
    }

    return this.idl.instructions[index];
  }

  protected getAccounts(
    idlInstruction: IdlInstruction,
    transactionInstruction: TransactionInstruction
  ): DecodedAccount[] {
    const accounts = Array.isArray(idlInstruction.accounts)
      ? idlInstruction.accounts
      : [idlInstruction.accounts];
    return accounts.map((def: IdlAccountItem, i): DecodedAccount => {
      if (!transactionInstruction.keys[i]) {
        return {
          message: `Account ${i} is missing`,
        };
      }
      return Object.assign(
        {
          formattedName: startCase(def.name),
          pubkey: transactionInstruction.keys[i].pubkey,
        },
        def
      );
    });
  }
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
