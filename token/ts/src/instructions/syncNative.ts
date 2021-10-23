import { struct, u8 } from '@solana/buffer-layout';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { TokenInstruction } from './types';

const dataLayout = struct<{ instruction: TokenInstruction }>([u8('instruction')]);

/**
 * Construct a SyncNative instruction
 *
 * @param nativeAccount Account to sync lamports from
 * @param programId     SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createSyncNativeInstruction(
    nativeAccount: PublicKey,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = [{ pubkey: nativeAccount, isSigner: false, isWritable: true }];

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: TokenInstruction.SyncNative }, data);

    return new TransactionInstruction({ keys, programId, data });
}
