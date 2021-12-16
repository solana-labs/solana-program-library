import { struct, u8 } from '@solana/buffer-layout';
import { PublicKey, SYSVAR_RENT_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { TokenInstruction } from './types';

const dataLayout = struct<{ instruction: TokenInstruction }>([u8('instruction')]);

/**
 * Construct an InitializeAccount instruction
 *
 * @param account   New token account
 * @param mint      Mint account
 * @param owner     Owner of the new account
 * @param programId SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createInitializeAccountInstruction(
    account: PublicKey,
    mint: PublicKey,
    owner: PublicKey,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = [
        { pubkey: account, isSigner: false, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: owner, isSigner: false, isWritable: false },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ];

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: TokenInstruction.InitializeAccount }, data);

    return new TransactionInstruction({ keys, programId, data });
}
