import { struct, u8 } from '@solana/buffer-layout';
import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { addSigners } from './internal';
import { TokenInstruction } from './types';

// TODO: docs
export interface CloseAccountInstructionData {
    instruction: TokenInstruction.CloseAccount;
}

// TODO: docs
export const closeAccountInstructionDataLayout = struct<CloseAccountInstructionData>([u8('instruction')]);

/**
 * Construct a CloseAccount instruction
 *
 * @param account      Account to close
 * @param destination  Account to receive the remaining balance of the closed account
 * @param authority    Account close authority
 * @param multiSigners Signing accounts if `authority` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createCloseAccountInstruction(
    account: PublicKey,
    destination: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: destination, isSigner: false, isWritable: true },
        ],
        authority,
        multiSigners
    );

    const data = Buffer.alloc(closeAccountInstructionDataLayout.span);
    closeAccountInstructionDataLayout.encode({ instruction: TokenInstruction.CloseAccount }, data);

    return new TransactionInstruction({ keys, programId, data });
}
