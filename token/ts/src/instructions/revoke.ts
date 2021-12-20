import { struct, u8 } from '@solana/buffer-layout';
import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { addSigners } from './internal';
import { TokenInstruction } from './types';

/** TODO: docs */
export interface RevokeInstructionData {
    instruction: TokenInstruction.Revoke;
}

/** TODO: docs */
export const revokeInstructionDataLayout = struct<RevokeInstructionData>([u8('instruction')]);

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

    const data = Buffer.alloc(revokeInstructionDataLayout.span);
    revokeInstructionDataLayout.encode({ instruction: TokenInstruction.Revoke }, data);

    return new TransactionInstruction({ keys, programId, data });
}
