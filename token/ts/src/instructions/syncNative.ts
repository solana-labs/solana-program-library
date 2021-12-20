import { struct, u8 } from '@solana/buffer-layout';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { TokenInstruction } from './types';

/** TODO: docs */
export interface SyncNativeInstructionData {
    instruction: TokenInstruction.SyncNative;
}

/** TODO: docs */
export const syncNativeInstructionDataLayout = struct<SyncNativeInstructionData>([u8('instruction')]);

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

    const data = Buffer.alloc(syncNativeInstructionDataLayout.span);
    syncNativeInstructionDataLayout.encode({ instruction: TokenInstruction.SyncNative }, data);

    return new TransactionInstruction({ keys, programId, data });
}
