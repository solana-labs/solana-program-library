import type { ConfirmOptions, Connection, PublicKey, Signer, TransactionSignature } from '@solana/web3.js';
import { sendAndConfirmTransaction, Transaction } from '@solana/web3.js';
import { TOKEN_2022_PROGRAM_ID } from '../../constants.js';
import { createConfidentialTransferUpdateMintInstruction } from './instructions.js';
import type { PodElGamalPubkey } from 'solana-zk-token-sdk-experimental';

export async function updateMint(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    autoApproveNewAccounts: boolean,
    auditorElGamalPubkey: PodElGamalPubkey | null,
    authority: Signer,
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const transaction = new Transaction().add(
        createConfidentialTransferUpdateMintInstruction(
            mint,
            authority.publicKey,
            autoApproveNewAccounts,
            auditorElGamalPubkey,
            programId
        )
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, authority], confirmOptions);
}
