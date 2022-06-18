import {
    ConfirmOptions,
    Connection,
    Keypair,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    SystemProgram,
    Transaction,
} from '@solana/web3.js';
import { getSigners } from '../../actions/internal';
import { TOKEN_2022_PROGRAM_ID } from '../../constants';
import { createInitializeMintInstruction } from '../../instructions';
import { ExtensionType, getMintLen } from '../extensionType';
import {
    createInitializeInterestBearingMintInstruction,
    createUpdateRateInterestBearingMintInstruction,
} from './instructions';

export async function createInterestBearingMint(
    connection: Connection,
    payer: Signer,
    mintAuthority: PublicKey,
    freezeAuthority: PublicKey,
    rateAuthority: PublicKey,
    rate: number,
    decimals: number,
    keypair = Keypair.generate(),
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<PublicKey> {
    const mintLen = getMintLen([ExtensionType.InterestBearingMint]);
    const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);
    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: keypair.publicKey,
            space: mintLen,
            lamports,
            programId,
        }),
        createInitializeInterestBearingMintInstruction(keypair.publicKey, rateAuthority, rate, programId),
        createInitializeMintInstruction(keypair.publicKey, decimals, mintAuthority, freezeAuthority, programId)
    );
    await sendAndConfirmTransaction(connection, transaction, [payer, keypair], confirmOptions);
    return keypair.publicKey;
}

export async function updateRateInterestBearingMint(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    rateAuthority: Signer,
    rate: number,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<string> {
    const [rateAuthorityPublicKey, signers] = getSigners(rateAuthority, multiSigners);
    const transaction = new Transaction().add(
        createUpdateRateInterestBearingMintInstruction(mint, rateAuthorityPublicKey, rate, signers, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, rateAuthority, ...signers], confirmOptions);
}
