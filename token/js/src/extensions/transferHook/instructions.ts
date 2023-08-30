import { struct, u8 } from '@solana/buffer-layout';
import type { Commitment, Connection, PublicKey, Signer } from '@solana/web3.js';
import { TransactionInstruction } from '@solana/web3.js';
import { programSupportsExtensions, TOKEN_2022_PROGRAM_ID } from '../../constants.js';
import { TokenUnsupportedInstructionError } from '../../errors.js';
import { addSigners } from '../../instructions/internal.js';
import { TokenInstruction } from '../../instructions/types.js';
import { publicKey } from '@solana/buffer-layout-utils';
import { createTransferCheckedInstruction } from '../../instructions/index.js';
import { addExtraAccountsToInstruction } from './actions.js';
import { createTransferCheckedWithFeeInstruction } from '../transferFee/instructions.js';

export enum TransferHookInstruction {
    Initialize = 0,
    Update = 1,
}

/** Deserialized instruction for the initiation of an transfer hook */
export interface InitializeTransferHookInstructionData {
    instruction: TokenInstruction.TransferHookExtension;
    transferHookInstruction: TransferHookInstruction.Initialize;
    authority: PublicKey;
    transferHookProgramId: PublicKey;
}

/** The struct that represents the instruction data as it is read by the program */
export const initializeTransferHookInstructionData = struct<InitializeTransferHookInstructionData>([
    u8('instruction'),
    u8('transferHookInstruction'),
    publicKey('authority'),
    publicKey('transferHookProgramId'),
]);

/**
 * Construct an InitializeTransferHook instruction
 *
 * @param mint                  Token mint account
 * @param authority             Transfer hook authority account
 * @param transferHookProgramId Transfer hook program account
 * @param programId             SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createInitializeTransferHookInstruction(
    mint: PublicKey,
    authority: PublicKey,
    transferHookProgramId: PublicKey,
    programId: PublicKey
): TransactionInstruction {
    if (!programSupportsExtensions(programId)) {
        throw new TokenUnsupportedInstructionError();
    }
    const keys = [{ pubkey: mint, isSigner: false, isWritable: true }];

    const data = Buffer.alloc(initializeTransferHookInstructionData.span);
    initializeTransferHookInstructionData.encode(
        {
            instruction: TokenInstruction.TransferHookExtension,
            transferHookInstruction: TransferHookInstruction.Initialize,
            authority,
            transferHookProgramId,
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}

/** Deserialized instruction for the initiation of an transfer hook */
export interface UpdateTransferHookInstructionData {
    instruction: TokenInstruction.TransferHookExtension;
    transferHookInstruction: TransferHookInstruction.Update;
    transferHookProgramId: PublicKey;
}

/** The struct that represents the instruction data as it is read by the program */
export const updateTransferHookInstructionData = struct<UpdateTransferHookInstructionData>([
    u8('instruction'),
    u8('transferHookInstruction'),
    publicKey('transferHookProgramId'),
]);

/**
 * Construct an UpdateTransferHook instruction
 *
 * @param mint                  Mint to update
 * @param authority             The mint's transfer hook authority
 * @param transferHookProgramId The new transfer hook program account
 * @param signers               The signer account(s) for a multisig
 * @param tokenProgramId        SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createUpdateTransferHookInstruction(
    mint: PublicKey,
    authority: PublicKey,
    transferHookProgramId: PublicKey,
    multiSigners: (Signer | PublicKey)[] = [],
    programId = TOKEN_2022_PROGRAM_ID
): TransactionInstruction {
    if (!programSupportsExtensions(programId)) {
        throw new TokenUnsupportedInstructionError();
    }

    const keys = addSigners([{ pubkey: mint, isSigner: false, isWritable: true }], authority, multiSigners);
    const data = Buffer.alloc(updateTransferHookInstructionData.span);
    updateTransferHookInstructionData.encode(
        {
            instruction: TokenInstruction.TransferHookExtension,
            transferHookInstruction: TransferHookInstruction.Update,
            transferHookProgramId,
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}

/**
 * Construct an transferChecked instruction with extra accounts for transfer hook
 *
 * @param connection            Connection to use
 * @param source                Source account
 * @param mint                  Mint to update
 * @param destination           Destination account
 * @param authority             The mint's transfer hook authority
 * @param amount                The amount of tokens to transfer
 * @param decimals              Number of decimals in transfer amount
 * @param multiSigners          The signer account(s) for a multisig
 * @param commitment            Commitment to use
 * @param programId             SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export async function createTransferCheckedWithTransferHookInstruction(
    connection: Connection,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    authority: PublicKey,
    amount: bigint,
    decimals: number,
    multiSigners: (Signer | PublicKey)[] = [],
    commitment?: Commitment,
    programId = TOKEN_2022_PROGRAM_ID
) {
    const rawInstruction = createTransferCheckedInstruction(
        source,
        mint,
        destination,
        authority,
        amount,
        decimals,
        multiSigners,
        programId
    );

    const hydratedInstruction = await addExtraAccountsToInstruction(
        connection,
        rawInstruction,
        mint,
        commitment,
        programId
    );

    return hydratedInstruction;
}

/**
 * Construct an transferChecked instruction with extra accounts for transfer hook
 *
 * @param connection            Connection to use
 * @param source                Source account
 * @param mint                  Mint to update
 * @param destination           Destination account
 * @param authority             The mint's transfer hook authority
 * @param amount                The amount of tokens to transfer
 * @param decimals              Number of decimals in transfer amount
 * @param fee                   The calculated fee for the transfer fee extension
 * @param multiSigners          The signer account(s) for a multisig
 * @param commitment            Commitment to use
 * @param programId             SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export async function createTransferCheckedWithFeeAndTransferHookInstruction(
    connection: Connection,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    authority: PublicKey,
    amount: bigint,
    decimals: number,
    fee: bigint,
    multiSigners: (Signer | PublicKey)[] = [],
    commitment?: Commitment,
    programId = TOKEN_2022_PROGRAM_ID
) {
    const rawInstruction = createTransferCheckedWithFeeInstruction(
        source,
        mint,
        destination,
        authority,
        amount,
        decimals,
        fee,
        multiSigners,
        programId
    );

    const hydratedInstruction = await addExtraAccountsToInstruction(
        connection,
        rawInstruction,
        mint,
        commitment,
        programId
    );

    return hydratedInstruction;
}
