import { struct, u8 } from '@solana/buffer-layout';
import { PublicKey, TransactionInstruction, SystemProgram } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, NATIVE_MINT } from '../constants';
import { TokenInstruction } from './types';

/** TODO: docs */
export interface CreateNativeMintInstructionData {
    instruction: TokenInstruction.CreateNativeMint;
}

/** TODO: docs */
export const createNativeMintInstructionData = struct<CreateNativeMintInstructionData>([u8('instruction')]);

/**
 * Construct a CreateNativeMint instruction
 *
 * @param account   New token account
 * @param mint      Mint account
 * @param owner     Owner of the new account
 * @param programId SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createCreateNativeMintInstruction(
    payer: PublicKey,
    programId = TOKEN_PROGRAM_ID,
    nativeMintId = NATIVE_MINT
): TransactionInstruction {
    const keys = [
        { pubkey: payer, isSigner: true, isWritable: true },
        { pubkey: nativeMintId, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ];

    const data = Buffer.alloc(createNativeMintInstructionData.span);
    createNativeMintInstructionData.encode({ instruction: TokenInstruction.CreateNativeMint }, data);

    return new TransactionInstruction({ keys, programId, data });
}
