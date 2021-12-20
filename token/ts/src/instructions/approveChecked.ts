import { struct, u8 } from '@solana/buffer-layout';
import { u64 } from '@solana/buffer-layout-utils';
import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { addSigners } from './internal';
import { TokenInstruction } from './types';

/** TODO: docs */
export interface ApproveCheckedInstructionData {
    instruction: TokenInstruction.ApproveChecked;
    amount: bigint;
    decimals: number;
}

/** TODO: docs */
export const approveCheckedInstructionDataLayout = struct<ApproveCheckedInstructionData>([
    u8('instruction'),
    u64('amount'),
    u8('decimals'),
]);

/**
 * Construct an ApproveChecked instruction
 *
 * @param account      Account to set the delegate for
 * @param mint         Mint account
 * @param delegate     Account authorized to transfer of tokens from the account
 * @param owner        Owner of the account
 * @param amount       Maximum number of tokens the delegate may transfer
 * @param decimals     Number of decimals in approve amount
 * @param multiSigners Signing accounts if `owner` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createApproveCheckedInstruction(
    account: PublicKey,
    mint: PublicKey,
    delegate: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    decimals: number,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
            { pubkey: delegate, isSigner: false, isWritable: false },
        ],
        owner,
        multiSigners
    );

    const data = Buffer.alloc(approveCheckedInstructionDataLayout.span);
    approveCheckedInstructionDataLayout.encode(
        {
            instruction: TokenInstruction.ApproveChecked,
            amount: BigInt(amount),
            decimals,
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}
