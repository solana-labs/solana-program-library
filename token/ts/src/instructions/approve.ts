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
export interface ApproveInstructionData {
    instruction: TokenInstruction.Approve;
    amount: bigint;
}

/** TODO: docs */
export const approveInstructionData = struct<ApproveInstructionData>([u8('instruction'), u64('amount')]);

/**
 * Construct an Approve instruction
 *
 * @param account      Account to set the delegate for
 * @param delegate     Account authorized to transfer tokens from the account
 * @param owner        Owner of the account
 * @param amount       Maximum number of tokens the delegate may transfer
 * @param multiSigners Signing accounts if `owner` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createApproveInstruction(
    account: PublicKey,
    delegate: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: delegate, isSigner: false, isWritable: false },
        ],
        owner,
        multiSigners
    );

    const data = Buffer.alloc(approveInstructionData.span);
    approveInstructionData.encode(
        {
            instruction: TokenInstruction.Approve,
            amount: BigInt(amount),
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedApproveInstruction {
    instruction: TokenInstruction.Approve;
    account: AccountMeta;
    delegate: AccountMeta;
    owner: AccountMeta;
    multiSigners: AccountMeta[];
    amount: bigint;
}

/**
 * Decode a Approve instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeApproveInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedApproveInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [account, delegate, owner, ...multiSigners] = instruction.keys;
    if (!account || !delegate || !owner) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== approveInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = approveInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.Approve) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        account,
        delegate,
        owner,
        multiSigners,
        amount: data.amount,
    };
}
