import { struct, u8 } from '@solana/buffer-layout';
import { u64 } from '@solana/buffer-layout-utils';
import { AccountMeta, PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import {
    TokenInvalidInstructionDataError,
    TokenInvalidInstructionKeysError,
    TokenInvalidInstructionProgramError,
    TokenInvalidInstructionTypeError,
} from '../errors';
import { addSigners } from './internal';
import { TokenInstruction } from './types';

/** TODO: docs */
export interface BurnInstructionData {
    instruction: TokenInstruction.Burn;
    amount: bigint;
}

/** TODO: docs */
export const burnInstructionData = struct<BurnInstructionData>([u8('instruction'), u64('amount')]);

/**
 * Construct a Burn instruction
 *
 * @param account      Account to burn tokens from
 * @param mint         Mint for the account
 * @param owner        Owner of the account
 * @param amount       Number of tokens to burn
 * @param multiSigners Signing accounts if `owner` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createBurnInstruction(
    account: PublicKey,
    mint: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: true },
        ],
        owner,
        multiSigners
    );

    const data = Buffer.alloc(burnInstructionData.span);
    burnInstructionData.encode(
        {
            instruction: TokenInstruction.Burn,
            amount: BigInt(amount),
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedBurnInstruction {
    instruction: TokenInstruction.Burn;
    account: AccountMeta;
    mint: AccountMeta;
    owner: AccountMeta;
    multiSigners: AccountMeta[];
    amount: bigint;
}

/**
 * Decode a Burn instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeBurnInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedBurnInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [account, mint, owner, ...multiSigners] = instruction.keys;
    if (!account || !mint || !owner) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== burnInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = burnInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.Burn) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        account,
        mint,
        owner,
        multiSigners,
        amount: data.amount,
    };
}
