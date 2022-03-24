import { struct, u8 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import { AccountMeta, PublicKey, TransactionInstruction } from '@solana/web3.js';
import {
    TokenInvalidInstructionDataError,
    TokenInvalidInstructionKeysError,
    TokenInvalidInstructionProgramError,
    TokenInvalidInstructionTypeError,
} from '../errors';
import { TokenInstruction } from './types';

/** TODO: docs */

export interface InitializeTransferFeeAmountInstructionData {
    instruction: TokenInstruction.TransferFeeExtension;
    transferFeeAmount: Number;
}

export const initializeTransferFeeInstructionData = struct<InitializeTransferFeeAmountInstructionData>([u8('instruction'),
u8('amount')]);

export function createInitializeTransferFeeAmountInstruction(mint: PublicKey, amount: Number, programId: PublicKey): TransactionInstruction {
    const keys = [{ pubkey: mint, isSigner: false, isWritable: true }];
    const data = Buffer.alloc(initializeTransferFeeInstructionData.span);
    initializeTransferFeeInstructionData.encode({
        instruction: TokenInstruction.TransferFeeExtension,
        transferFeeAmount: amount

    },
        data);
    
    return new TransactionInstruction({ keys, programId, data });
}

export interface DecodedInitailizeTransferFeeAmountInstruction {
    programId: PublicKey;
    keys: {
        mint: AccountMeta;
    };
    data: {
        instruction: TokenInstruction.TransferFeeExtension;
        amount: Number
    }
}



export function decodeInitializeTransferFeeInstruction(instruction: TransactionInstruction, programId: PublicKey): DecodedInitailizeTransferFeeAmountInstruction{
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();
    if (instruction.data.length !== initializeTransferFeeInstructionData.span)
        throw new TokenInvalidInstructionDataError();

    const {
        keys: { mint },
        data,
    } = decodeInitializeTransferFeeInstructionUnchecked(instruction);
    if (data.instruction !== TokenInstruction.TransferFeeExtension)
        throw new TokenInvalidInstructionTypeError();
    if (!mint) throw new TokenInvalidInstructionKeysError();

    return {
        programId,
        keys: {
            mint,
        },
        data,
    };
}



export function decodeInitializeTransferFeeInstructionUnchecked({
    programId,
    keys: [mint],
    data,
}: TransactionInstruction): DecodedInitailizeTransferFeeAmountInstruction {
    const { instruction, transferFeeAmount } =
        initializeTransferFeeInstructionData.decode(data);

    return {
        programId,
        keys: {
            mint,
        },
        data: {
            instruction,
            amount: transferFeeAmount,
        },
    };
}