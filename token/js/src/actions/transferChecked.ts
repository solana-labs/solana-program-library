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
import { createTransferCheckedInstruction } from '../instructions/index';
import { getSigners } from './internal';

/**
 * Transfer tokens from one account to another, asserting the token mint and decimals
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param source         Source account
 * @param mint           Mint for the account
 * @param destination    Destination account
 * @param owner          Owner of the source account
 * @param amount         Number of tokens to transfer
 * @param decimals       Number of decimals in transfer amount
 * @param multiSigners   Signing accounts if `owner` is a multisig
 * @param confirmOptions Options for confirming the transaction
 * @param programId      SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function transferChecked(
    connection: Connection,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    owner: Signer | PublicKey,
    amount: number | bigint,
    decimals: number,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    const [ownerPublicKey, signers] = getSigners(owner, multiSigners);

    const transaction = new Transaction().add(
        createTransferCheckedInstruction(
            source,
            mint,
            destination,
            ownerPublicKey,
            amount,
            decimals,
            multiSigners,
            programId
        )
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}
