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
export interface RevokeInstructionData {
    instruction: TokenInstruction.Revoke;
}

/** TODO: docs */
export const revokeInstructionData = struct<RevokeInstructionData>([u8('instruction')]);

/**
 * Construct a Revoke instruction
 *
 * @param account      Address of the token account
 * @param owner        Owner of the account
 * @param multiSigners Signing accounts if `owner` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createRevokeInstruction(
    account: PublicKey,
    owner: PublicKey,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners([{ pubkey: account, isSigner: false, isWritable: true }], owner, multiSigners);

    const data = Buffer.alloc(revokeInstructionData.span);
    revokeInstructionData.encode({ instruction: TokenInstruction.Revoke }, data);

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedRevokeInstruction {
    instruction: TokenInstruction.Revoke;
    account: AccountMeta;
    owner: AccountMeta;
    multiSigners: AccountMeta[];
}

/**
 * Decode a Revoke instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeRevokeInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedRevokeInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [account, owner, ...multiSigners] = instruction.keys;
    if (!account || !owner) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== revokeInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = revokeInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.Revoke) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        account,
        owner,
        multiSigners,
    };
}
