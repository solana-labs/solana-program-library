import { struct, u8 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
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

/** TODO: docs */
export interface DecodedSetAuthorityInstruction {
    instruction: TokenInstruction.SetAuthority;
    account: AccountMeta;
    currentAuthority: AccountMeta;
    multiSigners: AccountMeta[];
    authorityType: AuthorityType;
    newAuthority: PublicKey | null;
}

/**
 * Decode a SetAuthority instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeSetAuthorityInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedSetAuthorityInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [account, currentAuthority, ...multiSigners] = instruction.keys;
    if (!account || !currentAuthority) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== setAuthorityInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = setAuthorityInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.SetAuthority) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        account,
        currentAuthority,
        multiSigners,
        authorityType: data.authorityType,
        newAuthority: data.newAuthorityOption ? data.newAuthority : null,
    };
}
