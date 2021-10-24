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
import { createMintToCheckedInstruction } from '../instructions';
import { getSigners } from './internal';

/**
 * Mint tokens to an account, asserting the token mint and decimals
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint for the account
 * @param destination    Address of the account to mint to
 * @param authority      Minting authority
 * @param multiSigners   Signing accounts if `authority` is a multisig
 * @param amount         Amount to mint
 * @param decimals       Number of decimals in amount to mint
 * @param confirmOptions Options for confirming the transaction
 * @param programId      SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function mintToChecked(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    destination: PublicKey,
    authority: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    const [authorityPublicKey, signers] = getSigners(authority, multiSigners);

    const transaction = new Transaction().add(
        createMintToCheckedInstruction(mint, destination, authorityPublicKey, multiSigners, amount, decimals, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}
