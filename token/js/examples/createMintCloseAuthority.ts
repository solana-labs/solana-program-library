import {
    closeAccount,
    createInitializeMintInstruction,
    createInitializeMintCloseAuthorityInstruction,
    getMintLen,
    ExtensionType,
    TOKEN_2022_PROGRAM_ID,
} from '../src';
import {
    clusterApiUrl,
    sendAndConfirmTransaction,
    Connection,
    Keypair,
    SystemProgram,
    Transaction,
    LAMPORTS_PER_SOL,
    PublicKey
} from '@solana/web3.js';

(async () => {
    const payer = Keypair.generate();

    const mintKeypair = Keypair.generate();
    const mint = mintKeypair.publicKey;
    const mintAuthority = Keypair.generate();
    const freezeAuthority = Keypair.generate();
    const closeAuthority = Keypair.generate();

    // const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');
    const connection = new Connection("http://127.0.0.1:8899", 'processed');

    const airdropSignature = await connection.requestAirdrop(payer.publicKey, 2 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction({ signature: airdropSignature, ...(await connection.getLatestBlockhash()) });

    const extensions = [ExtensionType.MintCloseAuthority];
    const mintLen = getMintLen(extensions);
    const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

    await (async () => {
        try {
            // Initialize that account as a Mint
            createInitializeMintInstruction(
                new PublicKey('5tpiMLGAmsp6aWRoRSrM43WBjtbVtCFxjcbZTVxv6k7Q'),
                9,
                new PublicKey('5tpiMLGAmsp6aWRoRSrM43WBjtbVtCFxjcbZTVxv6k7Q'),
                new PublicKey('5tpiMLGAmsp6aWRoRSrM43WBjtbVtCFxjcbZTVxv6k7Q'),
            );
        } catch (e) {
            debugger
        }
    })();

    let inst;
    try {
        inst = createInitializeMintInstruction(
            mint,
            9,
            mint,
            mint,
            TOKEN_2022_PROGRAM_ID
        )
    } catch (e) {
        console.log("Alllaa")
        console.log("Alllaa")
        console.log("Alllaa")
        console.log("Alllaa")
        console.log("Alllaa")
    }
    console.log("OK", inst, "!")

    // const transaction = new Transaction().add(
    //     SystemProgram.createAccount({
    //         fromPubkey: payer.publicKey,
    //         newAccountPubkey: mint,
    //         space: mintLen,
    //         lamports,
    //         programId: TOKEN_2022_PROGRAM_ID,
    //     }),
    //     createInitializeMintCloseAuthorityInstruction(mint, closeAuthority.publicKey, TOKEN_2022_PROGRAM_ID),
    //     createInitializeMintInstruction(
    //         mint,
    //         9,
    //         mint,
    //         mint,
    //         TOKEN_2022_PROGRAM_ID
    //     )
    // );
    // await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair], undefined);
    //
    // console.log(mint.toBase58());
    //
    // await closeAccount(connection, payer, mint, payer.publicKey, closeAuthority, [], undefined, TOKEN_2022_PROGRAM_ID);
})();
