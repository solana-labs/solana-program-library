import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { AuthorityType, createSetAuthorityInstruction } from '../instructions';

/**
 * Assign a new authority to the account
 *
 * @param account          Public key of the account
 * @param newAuthority     New authority of the account
 * @param authorityType    Type of authority to set
 * @param currentAuthority Current authority of the account
 * @param multiSigners     Signing accounts if `currentAuthority` is a multiSig
 *
 * @return Signature of the confirmed transaction
 */
export async function setAuthority(
    connection: Connection,
    payer: Signer,
    account: PublicKey,
    newAuthority: PublicKey | null,
    authorityType: AuthorityType,
    currentAuthority: Signer | PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let currentAuthorityPublicKey: PublicKey;
    let signers: Signer[];
    if (currentAuthority instanceof PublicKey) {
        currentAuthorityPublicKey = currentAuthority;
        signers = multiSigners;
    } else {
        currentAuthorityPublicKey = currentAuthority.publicKey;
        signers = [currentAuthority];
    }

    const transaction = new Transaction().add(
        createSetAuthorityInstruction(
            account,
            newAuthority,
            authorityType,
            currentAuthorityPublicKey,
            multiSigners,
            programId
        )
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}
