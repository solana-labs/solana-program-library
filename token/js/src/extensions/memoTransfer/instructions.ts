import { struct, u8 } from '@solana/buffer-layout';
import type { PublicKey, Signer } from '@solana/web3.js';
import { TransactionInstruction } from '@solana/web3.js';
import { programSupportsExtensions, TOKEN_2022_PROGRAM_ID } from '../../constants.js';
import { TokenUnsupportedInstructionError } from '../../errors.js';
import { TokenInstruction } from '../../instructions/types.js';

export enum MemoTransferInstruction {
    Enable = 0,
    Disable = 1,
}

/** TODO: docs */
export interface MemoTransferInstructionData {
    instruction: TokenInstruction.MemoTransferExtension;
    memoTransferInstruction: MemoTransferInstruction;
}

/** TODO: docs */
export const memoTransferInstructionData = struct<MemoTransferInstructionData>([
    u8('instruction'),
    u8('memoTransferInstruction'),
]);

/**
 * Construct an EnableRequiredMemoTransfers instruction
 *
 * @param account         Token account to update
 * @param authority       The account's owner/delegate
 * @param signers         The signer account(s)
 * @param programId       SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createEnableRequiredMemoTransfersInstruction(
    account: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[] = [],
    programId = TOKEN_2022_PROGRAM_ID
): TransactionInstruction {
    return createMemoTransferInstruction(/* enable */ true, account, authority, multiSigners, programId);
}

/**
 * Construct a DisableMemoTransfer instruction
 *
 * @param account         Token account to update
 * @param authority       The account's owner/delegate
 * @param signers         The signer account(s)
 * @param programId       SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createDisableRequiredMemoTransfersInstruction(
    account: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[] = [],
    programId = TOKEN_2022_PROGRAM_ID
): TransactionInstruction {
    return createMemoTransferInstruction(/* enable */ false, account, authority, multiSigners, programId);
}

function createMemoTransferInstruction(
    enable: boolean,
    account: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[],
    programId: PublicKey
): TransactionInstruction {
    if (!programSupportsExtensions(programId)) {
        throw new TokenUnsupportedInstructionError();
    }
    const keys = [{ pubkey: account, isSigner: false, isWritable: true }];
    keys.push({ pubkey: authority, isSigner: !multiSigners.length, isWritable: false });
    for (const signer of multiSigners) {
        keys.push({ pubkey: signer.publicKey, isSigner: true, isWritable: false });
    }

    const data = Buffer.alloc(memoTransferInstructionData.span);
    memoTransferInstructionData.encode(
        {
            instruction: TokenInstruction.MemoTransferExtension,
            memoTransferInstruction: enable ? MemoTransferInstruction.Enable : MemoTransferInstruction.Disable,
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}
