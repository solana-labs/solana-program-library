import { struct, u8 } from '@solana/buffer-layout';
import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { TokenInstruction } from './types';
import { addSigners } from './utils';

const dataLayout = struct<{ instruction: TokenInstruction }>([u8('instruction')]);

/**
 * Construct a Revoke instruction
 *
 * @param account      Public key of the account
 * @param owner        Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param programId    SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createRevokeInstruction(
    account: PublicKey,
    owner: PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners([{ pubkey: account, isSigner: false, isWritable: true }], owner, multiSigners);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: TokenInstruction.Revoke }, data);

    return new TransactionInstruction({ keys, programId, data });
}
