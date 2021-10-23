import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
    Transaction,
    TransactionInstruction,
} from '@solana/web3.js';
import { Account } from './account';
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from './constants';
import { FAILED_TO_FIND_ACCOUNT, getAccountInfo, INVALID_ACCOUNT_OWNER } from './info';

/**
 * Get the address for the associated token account
 *
 * @param mint Token mint account
 * @param owner Owner of the new account
 * @param allowOwnerOffCurve @TODO: document
 * @param programId SPL Token program account
 * @param associatedTokenProgramId SPL Associated Token program account
 * @return Public key of the associated token account
 */
export async function getAssociatedTokenAddress(
    mint: PublicKey,
    owner: PublicKey,
    allowOwnerOffCurve = false,
    programId = TOKEN_PROGRAM_ID,
    associatedTokenProgramId = ASSOCIATED_TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    if (!allowOwnerOffCurve && !PublicKey.isOnCurve(owner.toBuffer())) {
        throw new Error(`Owner cannot sign: ${owner.toString()}`);
    }

    const [address] = await PublicKey.findProgramAddress(
        [owner.toBuffer(), programId.toBuffer(), mint.toBuffer()],
        associatedTokenProgramId
    );

    return address;
}

/**
 * Create and initialize the associated account.
 *
 * This account may then be used as a `transfer()` or `approve()` destination
 *
 * @param owner User account that will own the new account
 * @return Public key of the new associated account
 */
export async function createAssociatedTokenAccount(
    connection: Connection,
    mint: PublicKey,
    owner: PublicKey,
    payer: Signer,
    programId = TOKEN_PROGRAM_ID,
    associatedTokenProgramId = ASSOCIATED_TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    const associatedAddress = await getAssociatedTokenAddress(mint, owner, false, programId, associatedTokenProgramId);

    const transaction = new Transaction().add(
        createAssociatedTokenAccountInstruction(
            associatedTokenProgramId,
            programId,
            mint,
            associatedAddress,
            owner,
            payer.publicKey
        )
    );

    await sendAndConfirmTransaction(connection, transaction, [payer]);

    return associatedAddress;
}

/**
 * Retrieve the associated account or create one if not found.
 *
 * This account may then be used as a `transfer()` or `approve()` destination
 *
 * @param owner User account that will own the new account
 * @return The new associated account
 */
export async function getOrCreateAssociatedAccountInfo(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    programId = ASSOCIATED_TOKEN_PROGRAM_ID,
    tokenProgramId = TOKEN_PROGRAM_ID
): Promise<Account> {
    const associatedAddress = await getAssociatedTokenAddress(mint, owner, false, programId, tokenProgramId);

    // This is the optimum logic, considering TX fee, client-side computation,
    // RPC roundtrips and guaranteed idempotent.
    // Sadly we can't do this atomically;
    try {
        return await getAccountInfo(connection, mint, associatedAddress);
    } catch (err: any) {
        // INVALID_ACCOUNT_OWNER can be possible if the associatedAddress has
        // already been received some lamports (= became system accounts).
        // Assuming program derived addressing is safe, this is the only case
        // for the INVALID_ACCOUNT_OWNER in this code-path
        if (err?.message === FAILED_TO_FIND_ACCOUNT || err?.message === INVALID_ACCOUNT_OWNER) {
            // as this isn't atomic, it's possible others can create associated
            // accounts meanwhile
            try {
                const transaction = new Transaction().add(
                    createAssociatedTokenAccountInstruction(
                        programId,
                        tokenProgramId,
                        mint,
                        associatedAddress,
                        owner,
                        payer.publicKey
                    )
                );

                await sendAndConfirmTransaction(connection, transaction, [payer]);
            } catch (err: any) {
                // ignore all errors; for now there is no API compatible way to
                // selectively ignore the expected instruction error if the
                // associated account is existing already.
            }

            // Now this should always succeed
            return await getAccountInfo(connection, mint, associatedAddress);
        } else {
            throw err;
        }
    }
}

/**
 * Construct the AssociatedTokenProgram instruction to create the associated
 * token account
 *
 * @param mint Token mint account
 * @param associatedAccount New associated account
 * @param owner Owner of the new account
 * @param payer Payer of fees
 * @param programId SPL Token program account
 * @param associatedTokenProgramId SPL Associated Token program account
 */
export function createAssociatedTokenAccountInstruction(
    mint: PublicKey,
    associatedAccount: PublicKey,
    owner: PublicKey,
    payer: PublicKey,
    programId = TOKEN_PROGRAM_ID,
    associatedTokenProgramId = ASSOCIATED_TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = [
        { pubkey: payer, isSigner: true, isWritable: true },
        { pubkey: associatedAccount, isSigner: false, isWritable: true },
        { pubkey: owner, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: programId, isSigner: false, isWritable: false },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ];

    return new TransactionInstruction({
        keys,
        programId: associatedTokenProgramId,
        data: Buffer.alloc(0),
    });
}
