import {
    ConfirmOptions,
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createBurnInstruction } from '../instructions/index';
import { getSigners } from './internal';

/**
 * Burn tokens from an account
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param account        Account to burn tokens from
 * @param mint           Mint for the account
 * @param owner          Account owner
 * @param amount         Amount to burn
 * @param multiSigners   Signing accounts if `owner` is a multisig
 * @param confirmOptions Options for confirming the transaction
 * @param programId      SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function burn(
    connection: Connection,
    payer: Signer,
    account: PublicKey,
    mint: PublicKey,
    owner: Signer | PublicKey,
    amount: number | bigint,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    const [ownerPublicKey, signers] = getSigners(owner, multiSigners);

    const transaction = new Transaction().add(
        createBurnInstruction(account, mint, ownerPublicKey, amount, multiSigners, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}
