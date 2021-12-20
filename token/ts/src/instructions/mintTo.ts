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
export interface MintToInstructionData {
    instruction: TokenInstruction.MintTo;
    amount: bigint;
}

/** TODO: docs */
export const mintToInstructionData = struct<MintToInstructionData>([u8('instruction'), u64('amount')]);

/**
 * Construct a MintTo instruction
 *
 * @param mint         Public key of the mint
 * @param destination  Address of the token account to mint to
 * @param authority    The mint authority
 * @param amount       Amount to mint
 * @param multiSigners Signing accounts if `authority` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createMintToInstruction(
    mint: PublicKey,
    destination: PublicKey,
    authority: PublicKey,
    amount: number | bigint,
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

    const data = Buffer.alloc(mintToInstructionData.span);
    mintToInstructionData.encode(
        {
            instruction: TokenInstruction.MintTo,
            amount: BigInt(amount),
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedMintToInstruction {
    instruction: TokenInstruction.MintTo;
    mint: AccountMeta;
    destination: AccountMeta;
    authority: AccountMeta;
    multiSigners: AccountMeta[];
    amount: bigint;
}

/**
 * Decode a MintTo instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeMintToInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedMintToInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [mint, destination, authority, ...multiSigners] = instruction.keys;
    if (!mint || !destination || !authority) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== mintToInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = mintToInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.MintTo) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        mint,
        destination,
        authority,
        multiSigners,
        amount: data.amount,
    };
}
