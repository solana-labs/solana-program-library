import { struct, u8 } from '@solana/buffer-layout';
import { AccountMeta, PublicKey, SYSVAR_RENT_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import {
    TokenInvalidInstructionDataError,
    TokenInvalidInstructionKeysError,
    TokenInvalidInstructionProgramError,
    TokenInvalidInstructionTypeError,
} from '../errors';
import { TokenInstruction } from './types';

/** TODO: docs */
export interface InitializeAccountInstructionData {
    instruction: TokenInstruction.InitializeAccount;
}

/** TODO: docs */
export const initializeAccountInstructionData = struct<InitializeAccountInstructionData>([u8('instruction')]);

/**
 * Construct an InitializeAccount instruction
 *
 * @param account   New token account
 * @param mint      Mint account
 * @param owner     Owner of the new account
 * @param programId SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createInitializeAccountInstruction(
    account: PublicKey,
    mint: PublicKey,
    owner: PublicKey,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = [
        { pubkey: account, isSigner: false, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: owner, isSigner: false, isWritable: false },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ];

    const data = Buffer.alloc(initializeAccountInstructionData.span);
    initializeAccountInstructionData.encode({ instruction: TokenInstruction.InitializeAccount }, data);

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedInitializeAccountInstruction {
    instruction: TokenInstruction.InitializeAccount;
    account: AccountMeta;
    mint: AccountMeta;
    owner: AccountMeta;
    rent: AccountMeta;
    multiSigners: AccountMeta[];
}

/**
 * Decode an InitializeAccount instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeInitializeAccountInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedInitializeAccountInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [account, mint, owner, rent, ...multiSigners] = instruction.keys;
    if (!account || !mint || !owner || !rent) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== initializeAccountInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = initializeAccountInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.InitializeAccount) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        account,
        mint,
        owner,
        rent,
        multiSigners,
    };
}
