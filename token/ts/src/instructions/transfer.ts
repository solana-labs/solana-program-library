import { struct, u8 } from '@solana/buffer-layout';
import { u64 } from '@solana/buffer-layout-utils';
import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { addSigners } from './internal';
import { TokenInstruction } from './types';

// TODO: docs
export interface TransferInstructionData {
    instruction: TokenInstruction.Transfer;
    amount: bigint;
}

// TODO: docs
export const transferInstructionDataLayout = struct<TransferInstructionData>([u8('instruction'), u64('amount')]);

/**
 * Construct a Transfer instruction
 *
 * @param source       Source account
 * @param destination  Destination account
 * @param owner        Owner of the source account
 * @param amount       Number of tokens to transfer
 * @param multiSigners Signing accounts if `owner` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createTransferInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: source, isSigner: false, isWritable: true },
            { pubkey: destination, isSigner: false, isWritable: true },
        ],
        owner,
        multiSigners
    );

    const data = Buffer.alloc(transferInstructionDataLayout.span);
    transferInstructionDataLayout.encode(
        {
            instruction: TokenInstruction.Transfer,
            amount: BigInt(amount),
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}
