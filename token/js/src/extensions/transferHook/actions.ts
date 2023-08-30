import type { Commitment, ConfirmOptions, Connection, Signer, TransactionSignature } from '@solana/web3.js';
import { TransactionInstruction } from '@solana/web3.js';
import type { PublicKey } from '@solana/web3.js';
import { sendAndConfirmTransaction, Transaction } from '@solana/web3.js';
import { getSigners } from '../../actions/internal.js';
import { TOKEN_2022_PROGRAM_ID, programSupportsExtensions } from '../../constants.js';
import {
    createInitializeTransferHookInstruction,
    createTransferCheckedWithFeeAndTransferHookInstruction,
    createTransferCheckedWithTransferHookInstruction,
    createUpdateTransferHookInstruction,
} from './instructions.js';
import { getExtraAccountMetaAccount, getExtraAccountMetas, getTransferHook, resolveExtraAccountMeta } from './state.js';
import { getMint } from '../../state/index.js';
import { TokenUnsupportedInstructionError } from '../../errors.js';

/**
 * Initialize a transfer hook on a mint
 *
 * @param connection            Connection to use
 * @param payer                 Payer of the transaction fees
 * @param mint                  Mint to initialize with extension
 * @param authority             Transfer hook authority account
 * @param transferHookProgramId The transfer hook program account
 * @param confirmOptions        Options for confirming the transaction
 * @param programId             SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function initializeTransferHook(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    authority: PublicKey,
    transferHookProgramId: PublicKey,
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const transaction = new Transaction().add(
        createInitializeTransferHookInstruction(mint, authority, transferHookProgramId, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer], confirmOptions);
}

/**
 * Update the transfer hook program on a mint
 *
 * @param connection            Connection to use
 * @param payer                 Payer of the transaction fees
 * @param mint                  Mint to modify
 * @param transferHookProgramId New transfer hook program account
 * @param authority             Transfer hook update authority
 * @param multiSigners          Signing accounts if `freezeAuthority` is a multisig
 * @param confirmOptions        Options for confirming the transaction
 * @param programId             SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function updateTransferHook(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    transferHookProgramId: PublicKey,
    authority: Signer | PublicKey,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const [authorityPublicKey, signers] = getSigners(authority, multiSigners);

    const transaction = new Transaction().add(
        createUpdateTransferHookInstruction(mint, authorityPublicKey, transferHookProgramId, signers, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}

/**
 * Add extra accounts needed for transfer hook to an instruction
 *
 * @param connection      Connection to use
 * @param instruction     The transferChecked instruction to add accounts to
 * @param commitment      Commitment to use
 * @param programId       SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export async function addExtraAccountsToInstruction(
    connection: Connection,
    instruction: TransactionInstruction,
    mint: PublicKey,
    commitment?: Commitment,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionInstruction> {
    if (!programSupportsExtensions(programId)) {
        throw new TokenUnsupportedInstructionError();
    }

    const mintInfo = await getMint(connection, mint, commitment, programId);
    const transferHook = getTransferHook(mintInfo);
    if (transferHook == null) {
        return instruction;
    }

    const extraAccountsAccount = getExtraAccountMetaAccount(transferHook.programId, mint);
    const extraAccountsInfo = await connection.getAccountInfo(extraAccountsAccount, commitment);
    if (extraAccountsInfo == null) {
        return instruction;
    }

    const extraAccountMetas = getExtraAccountMetas(extraAccountsInfo);

    const accountMetas = instruction.keys;

    for (const extraAccountMeta of extraAccountMetas) {
        const accountMeta = resolveExtraAccountMeta(
            extraAccountMeta,
            accountMetas,
            instruction.data,
            transferHook.programId
        );
        accountMetas.push(accountMeta);
    }

    return new TransactionInstruction({ keys: accountMetas, programId, data: instruction.data });
}

/**
 * Transfer tokens from one account to another, asserting the token mint, and decimals
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param source         Source account
 * @param mint           Mint for the account
 * @param destination    Destination account
 * @param authority      Authority of the source account
 * @param amount         Number of tokens to transfer
 * @param decimals       Number of decimals in transfer amount
 * @param multiSigners   Signing accounts if `owner` is a multisig
 * @param confirmOptions Options for confirming the transaction
 * @param programId      SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function transferCheckedWithTransferHook(
    connection: Connection,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    authority: Signer | PublicKey,
    amount: bigint,
    decimals: number,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const [authorityPublicKey, signers] = getSigners(authority, multiSigners);

    const transaction = new Transaction().add(
        await createTransferCheckedWithTransferHookInstruction(
            connection,
            source,
            mint,
            destination,
            authorityPublicKey,
            amount,
            decimals,
            signers,
            confirmOptions?.commitment,
            programId
        )
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}

/**
 * Transfer tokens from one account to another, asserting the transfer fee, token mint, and decimals
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param source         Source account
 * @param mint           Mint for the account
 * @param destination    Destination account
 * @param authority      Authority of the source account
 * @param amount         Number of tokens to transfer
 * @param decimals       Number of decimals in transfer amount
 * @param fee            The calculated fee for the transfer fee extension
 * @param multiSigners   Signing accounts if `owner` is a multisig
 * @param confirmOptions Options for confirming the transaction
 * @param programId      SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function transferCheckedWithFeeAndTransferHook(
    connection: Connection,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    authority: Signer | PublicKey,
    amount: bigint,
    decimals: number,
    fee: bigint,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const [authorityPublicKey, signers] = getSigners(authority, multiSigners);

    const transaction = new Transaction().add(
        await createTransferCheckedWithFeeAndTransferHookInstruction(
            connection,
            source,
            mint,
            destination,
            authorityPublicKey,
            amount,
            decimals,
            fee,
            signers,
            confirmOptions?.commitment,
            programId
        )
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}
