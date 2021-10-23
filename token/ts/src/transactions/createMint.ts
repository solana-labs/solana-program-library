import {
    Connection,
    Keypair,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    SystemProgram,
    Transaction,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createInitializeMintInstruction } from '../instructions';
import { getMinimumBalanceForRentExemptMint, MINT_LEN } from '../state';

/**
 * Create and initialize a mint
 *
 * @param connection      Connection to use
 * @param payer           Fee payer for transaction
 * @param mintAuthority   Account or multisig that will control minting
 * @param freezeAuthority Optional account or multisig that can freeze token accounts
 * @param decimals        Location of the decimal place
 * @param programId       Optional token programId, uses the system programId by default
 *
 * @return Address of the new mint
 */
export async function createMint(
    connection: Connection,
    payer: Signer,
    mintAuthority: PublicKey,
    freezeAuthority: PublicKey | null,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    const lamports = await getMinimumBalanceForRentExemptMint(connection);

    const mintAccount = Keypair.generate();

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: mintAccount.publicKey,
            space: MINT_LEN,
            lamports,
            programId,
        }),
        createInitializeMintInstruction(mintAccount.publicKey, decimals, mintAuthority, freezeAuthority, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, mintAccount]);

    return mintAccount.publicKey;
}
