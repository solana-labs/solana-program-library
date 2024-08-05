import {
    clusterApiUrl,
    sendAndConfirmTransaction,
    Connection,
    Keypair,
    PublicKey,
    SystemProgram,
    Transaction,
    LAMPORTS_PER_SOL,
} from '@solana/web3.js';

import {
    ExtensionType,
    createInitializeMintInstruction,
    createInitializeTransferHookInstruction,
    getMintLen,
    TOKEN_2022_PROGRAM_ID,
    updateTransferHook,
    transferCheckedWithTransferHook,
    getAssociatedTokenAddressSync,
    ASSOCIATED_TOKEN_PROGRAM_ID,
} from '../src';

(async () => {
    const payer = Keypair.generate();

    const mintAuthority = Keypair.generate();
    const mintKeypair = Keypair.generate();
    const mint = mintKeypair.publicKey;

    const sender = Keypair.generate();
    const recipient = Keypair.generate();

    const extensions = [ExtensionType.TransferHook];
    const mintLen = getMintLen(extensions);
    const decimals = 9;
    const transferHookPogramId = new PublicKey('7N4HggYEJAtCLJdnHGCtFqfxcB5rhQCsQTze3ftYstVj');
    const newTransferHookProgramId = new PublicKey('7N4HggYEJAtCLJdnHGCtFqfxcB5rhQCsQTze3ftYstVj');

    const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');

    const airdropSignature = await connection.requestAirdrop(payer.publicKey, 2 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction({ signature: airdropSignature, ...(await connection.getLatestBlockhash()) });

    const mintLamports = await connection.getMinimumBalanceForRentExemption(mintLen);
    const mintTransaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: mint,
            space: mintLen,
            lamports: mintLamports,
            programId: TOKEN_2022_PROGRAM_ID,
        }),
        createInitializeTransferHookInstruction(mint, payer.publicKey, transferHookPogramId, TOKEN_2022_PROGRAM_ID),
        createInitializeMintInstruction(mint, decimals, mintAuthority.publicKey, null, TOKEN_2022_PROGRAM_ID),
    );
    await sendAndConfirmTransaction(connection, mintTransaction, [payer, mintKeypair], undefined);

    await updateTransferHook(
        connection,
        payer,
        mint,
        newTransferHookProgramId,
        payer.publicKey,
        [],
        undefined,
        TOKEN_2022_PROGRAM_ID,
    );

    const senderAta = getAssociatedTokenAddressSync(
        mint,
        sender.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    const recipientAta = getAssociatedTokenAddressSync(
        mint,
        recipient.publicKey,
        false,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
    );

    await transferCheckedWithTransferHook(
        connection,
        payer,
        senderAta,
        mint,
        recipientAta,
        sender,
        BigInt(1000000000),
        9,
        [],
        undefined,
        TOKEN_2022_PROGRAM_ID,
    );
})();
