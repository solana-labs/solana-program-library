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
export interface CloseAccountInstructionData {
    instruction: TokenInstruction.CloseAccount;
}

/** TODO: docs */
export const closeAccountInstructionData = struct<CloseAccountInstructionData>([u8('instruction')]);

/**
 * Construct a CloseAccount instruction
 *
 * @param account      Account to close
 * @param destination  Account to receive the remaining balance of the closed account
 * @param authority    Account close authority
 * @param multiSigners Signing accounts if `authority` is a multisig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createCloseAccountInstruction(
    account: PublicKey,
    destination: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: destination, isSigner: false, isWritable: true },
        ],
        authority,
        multiSigners
    );

    const data = Buffer.alloc(closeAccountInstructionData.span);
    closeAccountInstructionData.encode({ instruction: TokenInstruction.CloseAccount }, data);

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedCloseAccountInstruction {
    instruction: TokenInstruction.CloseAccount;
    account: AccountMeta;
    destination: AccountMeta;
    authority: AccountMeta;
    multiSigners: AccountMeta[];
}

/**
 * Decode a CloseAccount instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded instruction
 */
export function decodeCloseAccountInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedCloseAccountInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [account, destination, authority, ...multiSigners] = instruction.keys;
    if (!account || !destination || !authority) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== closeAccountInstructionData.span) throw new TokenInvalidInstructionTypeError();
    const data = closeAccountInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.CloseAccount) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        account,
        destination,
        authority,
        multiSigners,
    };
}
