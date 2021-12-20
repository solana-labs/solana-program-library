import { u8 } from '@solana/buffer-layout';
import { TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import {
    TokenInvalidInstructionDataError,
    TokenInvalidInstructionProgramError,
    TokenInvalidInstructionTypeError,
} from '../errors';
import { DecodedTransferInstruction, decodeTransferInstruction } from './transfer';
import { DecodedTransferCheckedInstruction, decodeTransferCheckedInstruction } from './transferChecked';

/** Instructions defined by the program */
export enum TokenInstruction {
    InitializeMint = 0,
    InitializeAccount = 1,
    InitializeMultisig = 2,
    Transfer = 3,
    Approve = 4,
    Revoke = 5,
    SetAuthority = 6,
    MintTo = 7,
    Burn = 8,
    CloseAccount = 9,
    FreezeAccount = 10,
    ThawAccount = 11,
    TransferChecked = 12,
    ApproveChecked = 13,
    MintToChecked = 14,
    BurnChecked = 15,
    InitializeAccount2 = 16,
    SyncNative = 17,
    InitializeAccount3 = 18,
    InitializeMultisig2 = 19,
    InitializeMint2 = 20,
}

/** TODO: docs */
export function decodeInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedTransferInstruction | DecodedTransferCheckedInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();
    if (!instruction.data.length) throw new TokenInvalidInstructionDataError();

    const type = u8().decode(instruction.data);
    if (type === TokenInstruction.Transfer) {
        return decodeTransferInstruction(instruction);
    }
    if (type === TokenInstruction.TransferChecked) {
        return decodeTransferCheckedInstruction(instruction);
    }
    // TODO: complete

    throw new TokenInvalidInstructionTypeError();
}
