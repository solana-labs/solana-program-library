import { struct, u8 } from '@solana/buffer-layout';
import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { addSigners } from './internal';
import { TokenInstruction } from './types';

// TODO: docs
export interface FreezeAccountInstructionData {
    instruction: TokenInstruction.FreezeAccount;
}

// TODO: docs
export const freezeAccountInstructionDataLayout = struct<FreezeAccountInstructionData>([u8('instruction')]);

/**
 * Construct a FreezeAccount instruction
 *
 * @param account      Account to freeze
 * @param mint         Mint account
 * @param authority    Mint freeze authority
 * @param multiSigners Signing accounts if `authority` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createFreezeAccountInstruction(
    account: PublicKey,
    mint: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
        ],
        authority,
        multiSigners
    );

    const data = Buffer.alloc(freezeAccountInstructionDataLayout.span);
    freezeAccountInstructionDataLayout.encode({ instruction: TokenInstruction.FreezeAccount }, data);

    return new TransactionInstruction({ keys, programId, data });
}
