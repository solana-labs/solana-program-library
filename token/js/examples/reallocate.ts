import {
    clusterApiUrl,
    sendAndConfirmTransaction,
    Connection,
    Keypair,
    Transaction,
    LAMPORTS_PER_SOL,
} from '@solana/web3.js';
import {
    createAccount,
    createMint,
    createEnableRequiredMemoTransfersInstruction,
    createReallocateInstruction,
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

    const owner = Keypair.generate();
    const account = await createAccount(
        connection,
        payer,
        mint,
        owner.publicKey,
        undefined,
        undefined,
        TOKEN_2022_PROGRAM_ID,
    );

    const extensions = [ExtensionType.MemoTransfer];
    const transaction = new Transaction().add(
        createReallocateInstruction(
            account,
            payer.publicKey,
            extensions,
            owner.publicKey,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        ),
        createEnableRequiredMemoTransfersInstruction(account, owner.publicKey, [], TOKEN_2022_PROGRAM_ID),
    );
    await sendAndConfirmTransaction(connection, transaction, [payer, owner], undefined);
})();
