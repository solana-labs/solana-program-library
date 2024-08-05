import {
    clusterApiUrl,
    sendAndConfirmTransaction,
    Connection,
    Keypair,
    SystemProgram,
    Transaction,
    LAMPORTS_PER_SOL,
} from '@solana/web3.js';
import { createMemoInstruction } from '@solana/spl-memo';
import {
    createAssociatedTokenAccount,
    createMint,
    createEnableRequiredMemoTransfersInstruction,
    createInitializeAccountInstruction,
    createTransferInstruction,
    disableRequiredMemoTransfers,
    enableRequiredMemoTransfers,
    getAccountLen,
    mintTo,
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

    const accountLen = getAccountLen([ExtensionType.MemoTransfer]);
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
        createEnableRequiredMemoTransfersInstruction(destination, owner.publicKey, [], TOKEN_2022_PROGRAM_ID),
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, owner, destinationKeypair], undefined);

    await disableRequiredMemoTransfers(connection, payer, destination, owner, [], undefined, TOKEN_2022_PROGRAM_ID);

    await enableRequiredMemoTransfers(connection, payer, destination, owner, [], undefined, TOKEN_2022_PROGRAM_ID);

    const sourceTokenAccount = await createAssociatedTokenAccount(
        connection,
        payer,
        mint,
        payer.publicKey,
        undefined,
        TOKEN_2022_PROGRAM_ID,
    );
    await mintTo(connection, payer, mint, sourceTokenAccount, mintAuthority, 100, [], undefined, TOKEN_2022_PROGRAM_ID);

    const transferTransaction = new Transaction().add(
        createMemoInstruction('Hello, memo-transfer!', [payer.publicKey]),
        createTransferInstruction(sourceTokenAccount, destination, payer.publicKey, 100, [], TOKEN_2022_PROGRAM_ID),
    );
    await sendAndConfirmTransaction(connection, transferTransaction, [payer], undefined);
})();
