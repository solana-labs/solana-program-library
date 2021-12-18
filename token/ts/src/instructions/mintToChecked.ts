import { struct, u8 } from '@solana/buffer-layout';
import { u64 } from '@solana/buffer-layout-utils';
import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { addSigners } from './internal';
import { TokenInstruction } from './types';

// TODO: docs
export interface MintToCheckedInstructionData {
    instruction: TokenInstruction.MintToChecked;
    amount: bigint;
    decimals: number;
}

// TODO: docs
export const mintToCheckedInstructionDataLayout = struct<MintToCheckedInstructionData>([
    u8('instruction'),
    u64('amount'),
    u8('decimals'),
]);

/**
 * Construct a MintToChecked instruction
 *
 * @param mint         Public key of the mint
 * @param destination  Address of the token account to mint to
 * @param authority    The mint authority
 * @param amount       Amount to mint
 * @param decimals     Number of decimals in amount to mint
 * @param multiSigners Signing accounts if `authority` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createMintToCheckedInstruction(
    mint: PublicKey,
    destination: PublicKey,
    authority: PublicKey,
    amount: number | bigint,
    decimals: number,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: mint, isSigner: false, isWritable: true },
            { pubkey: destination, isSigner: false, isWritable: true },
        ],
        authority,
        multiSigners
    );

    const data = Buffer.alloc(mintToCheckedInstructionDataLayout.span);
    mintToCheckedInstructionDataLayout.encode(
        {
            instruction: TokenInstruction.MintToChecked,
            amount: BigInt(amount),
            decimals,
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}
