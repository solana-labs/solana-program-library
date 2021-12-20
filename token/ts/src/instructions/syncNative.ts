import { struct, u8 } from '@solana/buffer-layout';
import { AccountMeta, PublicKey, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import {
    TokenInvalidInstructionDataError,
    TokenInvalidInstructionKeysError,
    TokenInvalidInstructionProgramError,
    TokenInvalidInstructionTypeError,
} from '../errors';
import { TokenInstruction } from './types';

/** TODO: docs */
export interface SyncNativeInstructionData {
    instruction: TokenInstruction.SyncNative;
}

/** TODO: docs */
export const syncNativeInstructionData = struct<SyncNativeInstructionData>([u8('instruction')]);

/**
 * Construct a SyncNative instruction
 *
 * @param account   Native account to sync lamports from
 * @param programId SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createSyncNativeInstruction(account: PublicKey, programId = TOKEN_PROGRAM_ID): TransactionInstruction {
    const keys = [{ pubkey: account, isSigner: false, isWritable: true }];

    const data = Buffer.alloc(syncNativeInstructionData.span);
    syncNativeInstructionData.encode({ instruction: TokenInstruction.SyncNative }, data);

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedSyncNativeInstruction {
    instruction: TokenInstruction.SyncNative;
    account: AccountMeta;
}

/**
 * Decode a SyncNative instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeSyncNativeInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedSyncNativeInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [account] = instruction.keys;
    if (!account) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== syncNativeInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = syncNativeInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.SyncNative) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        account,
    };
}
