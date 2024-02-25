import type { ConfirmOptions, Connection, PublicKey, Signer, TransactionSignature } from '@solana/web3.js';
import { sendAndConfirmTransaction, Transaction } from '@solana/web3.js';
import { createInitializeMemberInstruction } from '@solana/spl-token-group';

import { TOKEN_2022_PROGRAM_ID } from '../../constants.js';
import { getSigners } from '../../actions/internal.js';

/**
 * Initialize a new `Member` of a `Group`
 *
 * Assumes the `Group` has already been initialized,
 * as well as the mint for the member.
 *
 * @param connection             Connection to use
 * @param payer                  Payer of the transaction fees
 * @param mint                   Mint Account
 * @param memberMint             Mint Account for the member
 * @param memberMintAuthority    Mint Authority for the member
 * @param group                  Group Account
 * @param groupUpdateAuthority   Update Authority for the group
 * @param multiSigners           Signing accounts if `authority` is a multisig
 * @param confirmOptions         Options for confirming the transaction
 * @param programId              SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function tokenGroupInitializeMember(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    memberMint: PublicKey,
    memberMintAuthority: PublicKey,
    group: PublicKey,
    groupUpdateAuthority: PublicKey,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const [memberMintAuthorityPublicKey, signers] = getSigners(memberMintAuthority, multiSigners);

    const transaction = new Transaction().add(
        createInitializeMemberInstruction({
            programId,
            member: mint,
            memberMint,
            memberMintAuthority: memberMintAuthorityPublicKey,
            group,
            groupUpdateAuthority,
        })
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}
