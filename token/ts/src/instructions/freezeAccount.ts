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
export interface FreezeAccountInstructionData {
    instruction: TokenInstruction.FreezeAccount;
}

/** TODO: docs */
export const freezeAccountInstructionData = struct<FreezeAccountInstructionData>([u8('instruction')]);

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

    const data = Buffer.alloc(freezeAccountInstructionData.span);
    freezeAccountInstructionData.encode({ instruction: TokenInstruction.FreezeAccount }, data);

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedFreezeAccountInstruction {
    instruction: TokenInstruction.FreezeAccount;
    account: AccountMeta;
    mint: AccountMeta;
    authority: AccountMeta;
    multiSigners: AccountMeta[];
}

/**
 * Decode a FreezeAccount instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeFreezeAccountInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedFreezeAccountInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [account, mint, authority, ...multiSigners] = instruction.keys;
    if (!account || !mint || !authority) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== freezeAccountInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = freezeAccountInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.FreezeAccount) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        account,
        mint,
        authority,
        multiSigners,
    };
}
