import { struct, u8 } from '@solana/buffer-layout';
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
export interface ThawAccountInstructionData {
    instruction: TokenInstruction.ThawAccount;
}

/** TODO: docs */
export const thawAccountInstructionData = struct<ThawAccountInstructionData>([u8('instruction')]);

/**
 * Construct a ThawAccount instruction
 *
 * @param account      Account to thaw
 * @param mint         Mint account
 * @param authority    Mint freeze authority
 * @param multiSigners Signing accounts if `authority` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createThawAccountInstruction(
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

    const data = Buffer.alloc(thawAccountInstructionData.span);
    thawAccountInstructionData.encode({ instruction: TokenInstruction.ThawAccount }, data);

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedThawAccountInstruction {
    instruction: TokenInstruction.ThawAccount;
    account: AccountMeta;
    mint: AccountMeta;
    authority: AccountMeta;
    multiSigners: AccountMeta[];
}

/**
 * Decode a ThawAccount instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeThawAccountInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedThawAccountInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [account, mint, authority, ...multiSigners] = instruction.keys;
    if (!account || !mint || !authority) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== thawAccountInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = thawAccountInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.ThawAccount) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        account,
        mint,
        authority,
        multiSigners,
    };
}
