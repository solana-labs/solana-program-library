import { struct, u8 } from '@solana/buffer-layout';
import { u64 } from '@solana/buffer-layout-utils';
import { AccountMeta, PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import {
    TokenInvalidInstructionDataError,
    TokenInvalidInstructionKeysError,
    TokenInvalidInstructionProgramError,
    TokenInvalidInstructionTypeError,
} from '../errors';
import { addSigners } from './internal';
import { TokenInstruction } from './types';

/** TODO: docs */
export interface TransferInstructionData {
    instruction: TokenInstruction.Transfer;
    amount: bigint;
}

/** TODO: docs */
export const transferInstructionDataLayout = struct<TransferInstructionData>([u8('instruction'), u64('amount')]);

/**
 * Construct a Transfer instruction
 *
 * @param source       Source account
 * @param destination  Destination account
 * @param owner        Owner of the source account
 * @param amount       Number of tokens to transfer
 * @param multiSigners Signing accounts if `owner` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createTransferInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: source, isSigner: false, isWritable: true },
            { pubkey: destination, isSigner: false, isWritable: true },
        ],
        owner,
        multiSigners
    );

    const data = Buffer.alloc(transferInstructionDataLayout.span);
    transferInstructionDataLayout.encode(
        {
            instruction: TokenInstruction.Transfer,
            amount: BigInt(amount),
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedTransferInstruction {
    instruction: TokenInstruction.Transfer;
    source: AccountMeta;
    destination: AccountMeta;
    owner: AccountMeta;
    multiSigners: AccountMeta[];
    amount: bigint;
}

/**
 * Decode a Transfer instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 */
export function decodeTransferInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedTransferInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [source, destination, owner, ...multiSigners] = instruction.keys;
    if (!source || !destination || !owner) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== transferInstructionDataLayout.span) throw new TokenInvalidInstructionTypeError();
    const data = transferInstructionDataLayout.decode(instruction.data);
    if (data.instruction !== TokenInstruction.Transfer) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        source,
        destination,
        owner,
        multiSigners,
        amount: data.amount,
    };
}
