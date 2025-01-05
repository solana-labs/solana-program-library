import { struct, u8, Layout } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import type { AccountMeta, PublicKey } from '@solana/web3.js';
import { TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants.js';
import {
    TokenInvalidInstructionDataError,
    TokenInvalidInstructionKeysError,
    TokenInvalidInstructionProgramError,
    TokenInvalidInstructionTypeError,
} from '../errors.js';
import { TokenInstruction } from './types.js';
import { COptionPublicKeyLayout, getSetOfPossibleSpans } from '../serialization.js';

/** TODO: docs */
export interface InitializeMint2InstructionData {
    instruction: TokenInstruction.InitializeMint2;
    decimals: number;
    mintAuthority: PublicKey;
    freezeAuthority: PublicKey | null;
}

/** TODO: docs */
export const initializeMint2InstructionData = struct<InitializeMint2InstructionData>([
    u8('instruction'),
    u8('decimals'),
    publicKey('mintAuthority'),
    new COptionPublicKeyLayout('freezeAuthority'),
]);

const ALLOWED_SPANS = getSetOfPossibleSpans(initializeTransferFeeConfigInstructionData);

/**
 * Construct an InitializeMint2 instruction
 *
 * @param mint            Token mint account
 * @param decimals        Number of decimals in token account amounts
 * @param mintAuthority   Minting authority
 * @param freezeAuthority Optional authority that can freeze token accounts
 * @param programId       SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createInitializeMint2Instruction(
    mint: PublicKey,
    decimals: number,
    mintAuthority: PublicKey,
    freezeAuthority: PublicKey | null,
    programId = TOKEN_PROGRAM_ID,
): TransactionInstruction {
    const keys = [{ pubkey: mint, isSigner: false, isWritable: true }];

    const data = Buffer.alloc(initializeMint2InstructionData.span);
    initializeMint2InstructionData.encode(
        {
            instruction: TokenInstruction.InitializeMint2,
            decimals,
            mintAuthority,
            freezeAuthority,
        },
        data,
    );

    return new TransactionInstruction({ keys, programId, data });
}

/** A decoded, valid InitializeMint2 instruction */
export interface DecodedInitializeMint2Instruction {
    programId: PublicKey;
    keys: {
        mint: AccountMeta;
    };
    data: {
        instruction: TokenInstruction.InitializeMint2;
        decimals: number;
        mintAuthority: PublicKey;
        freezeAuthority: PublicKey | null;
    };
}

/**
 * Decode an InitializeMint2 instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
export function decodeInitializeMint2Instruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID,
): DecodedInitializeMint2Instruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();
    if (!ALLOWED_SPANS.has(instruction.data.length)) throw new TokenInvalidInstructionDataError();

    const {
        keys: { mint },
        data,
    } = decodeInitializeMint2InstructionUnchecked(instruction);
    if (data.instruction !== TokenInstruction.InitializeMint2) throw new TokenInvalidInstructionTypeError();
    if (!mint) throw new TokenInvalidInstructionKeysError();

    return {
        programId,
        keys: {
            mint,
        },
        data,
    };
}

/** A decoded, non-validated InitializeMint2 instruction */
export interface DecodedInitializeMint2InstructionUnchecked {
    programId: PublicKey;
    keys: {
        mint: AccountMeta | undefined;
    };
    data: {
        instruction: number;
        decimals: number;
        mintAuthority: PublicKey;
        freezeAuthority: PublicKey | null;
    };
}

/**
 * Decode an InitializeMint2 instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
export function decodeInitializeMint2InstructionUnchecked({
    programId,
    keys: [mint],
    data,
}: TransactionInstruction): DecodedInitializeMint2InstructionUnchecked {
    const { instruction, decimals, mintAuthority, freezeAuthority } = initializeMint2InstructionData.decode(data);

    return {
        programId,
        keys: {
            mint,
        },
        data: {
            instruction,
            decimals,
            mintAuthority,
            freezeAuthority,
        },
    };
}
