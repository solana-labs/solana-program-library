import { struct, u8 } from '@solana/buffer-layout';
import { PublicKey, SYSVAR_RENT_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { TokenInstruction } from './types';

// TODO: docs
export interface InitializeMultisigInstructionData {
    instruction: TokenInstruction.InitializeMultisig;
    m: number;
}

// TODO: docs
export const initializeMultisigInstructionDataLayout = struct<InitializeMultisigInstructionData>([
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

    const data = Buffer.alloc(initializeMultisigInstructionDataLayout.span);
    initializeMultisigInstructionDataLayout.encode(
        {
            instruction: TokenInstruction.InitializeMultisig,
            m,
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}
