import { struct, u8 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
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
export interface InitializeMintInstructionData {
    instruction: TokenInstruction.InitializeMint;
    decimals: number;
    mintAuthority: PublicKey;
    freezeAuthorityOption: 1 | 0;
    freezeAuthority: PublicKey;
}

/** TODO: docs */
export const initializeMintInstructionData = struct<InitializeMintInstructionData>([
    u8('instruction'),
    u8('decimals'),
    publicKey('mintAuthority'),
    u8('freezeAuthorityOption'),
    publicKey('freezeAuthority'),
]);

/**
 * Construct an InitializeMint instruction
 *
 * @param mint            Token mint account
 * @param decimals        Number of decimals in token account amounts
 * @param mintAuthority   Minting authority
 * @param freezeAuthority Optional authority that can freeze token accounts
 * @param programId       SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createInitializeMintInstruction(
    mint: PublicKey,
    decimals: number,
    mintAuthority: PublicKey,
    freezeAuthority: PublicKey | null,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = [
        { pubkey: mint, isSigner: false, isWritable: true },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ];

    const data = Buffer.alloc(initializeMintInstructionData.span);
    initializeMintInstructionData.encode(
        {
            instruction: TokenInstruction.InitializeMint,
            decimals,
            mintAuthority,
            freezeAuthorityOption: freezeAuthority ? 1 : 0,
            freezeAuthority: freezeAuthority || new PublicKey(0),
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedInitializeMintInstruction {
    instruction: TokenInstruction.InitializeMint;
    mint: AccountMeta;
    rent: AccountMeta;
    decimals: number;
    mintAuthority: PublicKey;
    freezeAuthority: PublicKey | null;
}

/**
 * Decode a InitializeMint instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeInitializeMintInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedInitializeMintInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [mint, rent] = instruction.keys;
    if (!mint || !rent) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== initializeMintInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = initializeMintInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.InitializeMint) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        mint,
        rent,
        decimals: data.decimals,
        mintAuthority: data.mintAuthority,
        freezeAuthority: data.freezeAuthorityOption ? data.freezeAuthority : null,
    };
}
