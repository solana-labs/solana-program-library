import { IdlField } from "@project-serum/anchor/dist/idl";
import { TransactionInstruction } from "@solana/web3.js";
import { startCase } from "../../program/util/helpers";
import { Coder, DecodedArgument, DecodedInstruction } from "../coder";
import { IdlInstruction } from "../idl";
import { fieldLayout } from "../util/borsh";
import * as borsh from "@project-serum/borsh";
import camelCase from "camelcase";

export class Borsh extends Coder {
  public decodeInstruction(
    instruction: TransactionInstruction
  ): DecodedInstruction {
    const { programId } = instruction;
    const index = this.getInstructionIndex(instruction.data);
    const idlIx = this.getInstruction(index);
    const { name } = idlIx;
    const formattedName = startCase(name);
    const accounts = this.getAccounts(idlIx, instruction);
    const args = this.getArguments(idlIx, instruction);

    return {
      name,
      formattedName,
      programId,
      accounts,
      args,
    };
  }

  private getInstructionIndex(data: Buffer) {
    return data.readUInt8(0);
  }

  private getArguments(
    idlInstruction: IdlInstruction,
    transactionInstruction: TransactionInstruction
  ): DecodedArgument[] {
    const coder = this.buildCoder(idlInstruction);
    const decoded = coder.decode(transactionInstruction.data.slice(1)); // skip over enum header

    return idlInstruction.args.map((field) => {
      const name = camelCase(field.name);

      if (!(name in decoded)) {
        return {
          message: `Field ${name} is missing`,
        };
      }

      return Object.assign(
        {
          formattedName: startCase(field.name),
          value: decoded[name],
        },
        field
      );
    });
  }

  private buildCoder(idlInstruction: IdlInstruction) {
    const fieldLayouts = idlInstruction.args.map((arg: IdlField) =>
      fieldLayout(arg, this.idl.types)
    );
    return borsh.struct(fieldLayouts);
  }
}
