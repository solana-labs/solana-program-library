import { struct, u8 } from '@solana/buffer-layout';
import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { TokenInstruction } from './types';
import { addSigners } from './utils';

const dataLayout = struct<{ instruction: TokenInstruction }>([u8('instruction')]);

/**
 * Construct a Close instruction
 *
 * @param account      Account to close
 * @param dest         Account to receive the remaining balance of the closed account
 * @param authority    Account close authority
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createCloseAccountInstruction(
    account: PublicKey,
    dest: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: dest, isSigner: false, isWritable: true },
        ],
        authority,
        multiSigners
    );

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: TokenInstruction.CloseAccount }, data);

    return new TransactionInstruction({ keys, programId, data });
}
