import { TransactionInstruction } from "@solana/web3.js";
import { Coder, DecodedArgument, DecodedInstruction } from "../coder";
import { sighash, SIGHASH_GLOBAL_NAMESPACE } from "../util/anchor";
import camelCase from "camelcase";
import { IdlError, IdlField, IdlInstruction } from "../idl";
import { startCase } from "../../program/util/helpers";
import { fieldLayout } from "../util/borsh";
import * as borsh from "@project-serum/borsh";

const SIGHASH_OFFSET = 8;

export class Anchor extends Coder {
  decodeInstruction(instruction: TransactionInstruction): DecodedInstruction {
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
    let i = 0;
    for (let ix of this.idl.instructions) {
      const shash = sighash(SIGHASH_GLOBAL_NAMESPACE, camelCase(ix.name));
      if (data.slice(0, SIGHASH_OFFSET).equals(shash)) {
        return i;
      }
      i++;
    }

    throw new IdlError("Instruction not found in IDL");
  }

  private getArguments(
    idlInstruction: IdlInstruction,
    transactionInstruction: TransactionInstruction
  ): DecodedArgument[] {
    const coder = this.buildCoder(idlInstruction);
    const decoded = coder.decode(
      transactionInstruction.data.slice(SIGHASH_OFFSET)
    ); // skip over enum header

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
