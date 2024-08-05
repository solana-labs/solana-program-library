import {
    clusterApiUrl,
    sendAndConfirmTransaction,
    Connection,
    Keypair,
    SystemProgram,
    Transaction,
    LAMPORTS_PER_SOL,
} from '@solana/web3.js';
import {
    AccountState,
    createInitializeMintInstruction,
    createInitializeDefaultAccountStateInstruction,
    getMintLen,
    updateDefaultAccountState,
    ExtensionType,
    TOKEN_2022_PROGRAM_ID,
} from '../src';

(async () => {
    const payer = Keypair.generate();

    const mintAuthority = Keypair.generate();
    const freezeAuthority = Keypair.generate();
    const mintKeypair = Keypair.generate();
    const mint = mintKeypair.publicKey;

    const extensions = [ExtensionType.DefaultAccountState];
    const mintLen = getMintLen(extensions);
    const decimals = 9;

    const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');

    const airdropSignature = await connection.requestAirdrop(payer.publicKey, 2 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction({ signature: airdropSignature, ...(await connection.getLatestBlockhash()) });

    const defaultState = AccountState.Frozen;

    const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);
    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: mint,
            space: mintLen,
            lamports,
            programId: TOKEN_2022_PROGRAM_ID,
        }),
        createInitializeDefaultAccountStateInstruction(mint, defaultState, TOKEN_2022_PROGRAM_ID),
        createInitializeMintInstruction(
            mint,
            decimals,
            mintAuthority.publicKey,
            freezeAuthority.publicKey,
            TOKEN_2022_PROGRAM_ID,
        ),
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair], undefined);

    await updateDefaultAccountState(
        connection,
        payer,
        mint,
        AccountState.Initialized,
        freezeAuthority,
        [],
        undefined,
        TOKEN_2022_PROGRAM_ID,
    );
})();
