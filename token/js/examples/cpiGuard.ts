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
    createMint,
    createEnableCpiGuardInstruction,
    createInitializeAccountInstruction,
    disableCpiGuard,
    enableCpiGuard,
    getAccountLen,
    ExtensionType,
    TOKEN_2022_PROGRAM_ID,
} from '../src';

(async () => {
    const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');

    const payer = Keypair.generate();
    const airdropSignature = await connection.requestAirdrop(payer.publicKey, 2 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction({ signature: airdropSignature, ...(await connection.getLatestBlockhash()) });

    const mintAuthority = Keypair.generate();
    const decimals = 9;
    const mint = await createMint(
        connection,
        payer,
        mintAuthority.publicKey,
        mintAuthority.publicKey,
        decimals,
        undefined,
        undefined,
        TOKEN_2022_PROGRAM_ID,
    );

    const accountLen = getAccountLen([ExtensionType.CpiGuard]);
    const lamports = await connection.getMinimumBalanceForRentExemption(accountLen);

    const owner = Keypair.generate();
    const destinationKeypair = Keypair.generate();
    const destination = destinationKeypair.publicKey;
    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: destination,
            space: accountLen,
            lamports,
            programId: TOKEN_2022_PROGRAM_ID,
        }),
        createInitializeAccountInstruction(destination, mint, owner.publicKey, TOKEN_2022_PROGRAM_ID),
        createEnableCpiGuardInstruction(destination, owner.publicKey, [], TOKEN_2022_PROGRAM_ID),
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, owner, destinationKeypair], undefined);

    await disableCpiGuard(connection, payer, destination, owner, [], undefined, TOKEN_2022_PROGRAM_ID);

    await enableCpiGuard(connection, payer, destination, owner, [], undefined, TOKEN_2022_PROGRAM_ID);
})();
