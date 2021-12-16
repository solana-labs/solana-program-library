import { struct, u8 } from '@solana/buffer-layout';
import { u64 } from '@solana/buffer-layout-utils';
import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { addSigners } from './internal';
import { TokenInstruction } from './types';

const dataLayout = struct<{
    instruction: TokenInstruction;
    amount: bigint;
}>([u8('instruction'), u64('amount')]);

/**
 * Construct an Approve instruction
 *
 * @param account      Account to set the delegate for
 * @param delegate     Account authorized to transfer tokens from the account
 * @param owner        Owner of the account
 * @param amount       Maximum number of tokens the delegate may transfer
 * @param multiSigners Signing accounts if `owner` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createApproveInstruction(
    account: PublicKey,
    delegate: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: delegate, isSigner: false, isWritable: false },
        ],
        owner,
        multiSigners
    );

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.Approve,
            amount: BigInt(amount),
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}
