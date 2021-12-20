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
export interface InitializeMultisigInstructionData {
    instruction: TokenInstruction.InitializeMultisig;
    m: number;
}

/** TODO: docs */
export const initializeMultisigInstructionData = struct<InitializeMultisigInstructionData>([
    u8('instruction'),
    u8('m'),
]);

/**
 * Construct an InitializeMultisig instruction
 *
 * @param account   Multisig account
 * @param signers   Full set of signers
 * @param m         Number of required signatures
 * @param programId SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createInitializeMultisigInstruction(
    account: PublicKey,
    signers: PublicKey[],
    m: number,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = [
        { pubkey: account, isSigner: false, isWritable: true },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ];
    for (const signer of signers) {
        keys.push({ pubkey: signer, isSigner: false, isWritable: false });
    }

    const data = Buffer.alloc(initializeMultisigInstructionData.span);
    initializeMultisigInstructionData.encode(
        {
            instruction: TokenInstruction.InitializeMultisig,
            m,
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}

/** TODO: docs */
export interface DecodedInitializeMultisigInstruction {
    instruction: TokenInstruction.InitializeMultisig;
    account: AccountMeta;
    rent: AccountMeta;
    signers: AccountMeta[];
    m: number;
}

/**
 * Decode a InitializeMultisig instruction
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program multisig
 *
 * @return Decoded instruction
 */
export function decodeInitializeMultisigInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedInitializeMultisigInstruction {
    if (!instruction.programId.equals(programId)) throw new TokenInvalidInstructionProgramError();

    const [account, rent, ...signers] = instruction.keys;
    if (!account || !rent || !signers.length) throw new TokenInvalidInstructionKeysError();

    if (instruction.data.length !== initializeMultisigInstructionData.span)
        throw new TokenInvalidInstructionTypeError();
    const data = initializeMultisigInstructionData.decode(instruction.data);
    if (data.instruction !== TokenInstruction.InitializeMultisig) throw new TokenInvalidInstructionDataError();

    return {
        instruction: data.instruction,
        account,
        rent,
        signers,
        m: data.m,
    };
}
