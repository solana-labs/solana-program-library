import { struct, u8 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import type { AccountMeta, Signer } from '@solana/web3.js';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants.js';
import {
    TokenInvalidInstructionDataError,
    TokenInvalidInstructionKeysError,
    TokenInvalidInstructionProgramError,
    TokenInvalidInstructionTypeError,
} from '../errors.js';
import { addSigners } from './internal.js';
import { TokenInstruction } from './types.js';

/** Authority types defined by the program */
export enum AuthorityType {
    MintTokens = 0,
    FreezeAccount = 1,
    AccountOwner = 2,
    CloseAccount = 3,
}

/** TODO: docs */
export interface SetAuthorityInstructionData {
    instruction: TokenInstruction.SetAuthority;
    authorityType: AuthorityType;
    newAuthorityOption: 1 | 0;
    newAuthority: PublicKey;
}

/** TODO: docs */
export const setAuthorityInstructionData = struct<SetAuthorityInstructionData>([
    u8('instruction'),
    u8('authorityType'),
    u8('newAuthorityOption'),
    publicKey('newAuthority'),
]);

/**
 * Construct a SetAuthority instruction
 *
 * @param account          Address of the token account
 * @param currentAuthority Current authority of the specified type
 * @param authorityType    Type of authority to set
 * @param newAuthority     New authority of the account
 * @param multiSigners     Signing accounts if `currentAuthority` is a multisig
 * @param programId        SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createSetAuthorityInstruction(
    account: PublicKey,
    currentAuthority: PublicKey,
    authorityType: AuthorityType,
    newAuthority: PublicKey | null,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners([{ pubkey: account, isSigner: false, isWritable: true }], currentAuthority, multiSigners);

    const data = Buffer.alloc(setAuthorityInstructionData.span);
    setAuthorityInstructionData.encode(
        {
            instruction: TokenInstruction.SetAuthority,
            authorityType,
            newAuthorityOption: newAuthority ? 1 : 0,
            newAuthority: newAuthority || new PublicKey(0),
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}

/** A decoded, valid SetAuthority instruction */
export interface DecodedSetAuthorityInstruction {
    programId: PublicKey;
    keys: {
        account: AccountMeta;
        currentAuthority: AccountMeta;
        multiSigners: AccountMeta[];
    };
    data: {
        instruction: TokenInstruction.SetAuthority;
        authorityType: AuthorityType;
        newAuthority: PublicKey | null;
    };
}

/**
 * Decode a SetAuthority instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
export function decodeSetAuthorityInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedSetAuthorityInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();
    if (instruction.data.length !== setAuthorityInstructionData.span) throw new TokenInvalidInstructionDataError();

    const {
        keys: { account, currentAuthority, multiSigners },
        data,
    } = decodeSetAuthorityInstructionUnchecked(instruction);
    if (data.instruction !== TokenInstruction.SetAuthority) throw new TokenInvalidInstructionTypeError();
    if (!account || !currentAuthority) throw new TokenInvalidInstructionKeysError();

    // TODO: key checks?

    return {
        programId,
        keys: {
            account,
            currentAuthority,
            multiSigners,
        },
        data,
    };
}

/** A decoded, non-validated SetAuthority instruction */
export interface DecodedSetAuthorityInstructionUnchecked {
    programId: PublicKey;
    keys: {
        account: AccountMeta | undefined;
        currentAuthority: AccountMeta | undefined;
        multiSigners: AccountMeta[];
    };
    data: {
        instruction: number;
        authorityType: AuthorityType;
        newAuthority: PublicKey | null;
    };
}

/**
 * Decode a SetAuthority instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
export function decodeSetAuthorityInstructionUnchecked({
    programId,
    keys: [account, currentAuthority, ...multiSigners],
    data,
}: TransactionInstruction): DecodedSetAuthorityInstructionUnchecked {
    const { instruction, authorityType, newAuthorityOption, newAuthority } = setAuthorityInstructionData.decode(data);

    return {
        programId,
        keys: {
            account,
            currentAuthority,
            multiSigners,
        },
        data: {
            instruction,
            authorityType,
            newAuthority: newAuthorityOption ? newAuthority : null,
        },
    };
}
