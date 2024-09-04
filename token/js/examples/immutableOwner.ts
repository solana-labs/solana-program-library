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
    createAccount,
    createMint,
    createInitializeImmutableOwnerInstruction,
    createInitializeAccountInstruction,
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

    const accountLen = getAccountLen([ExtensionType.ImmutableOwner]);
    const lamports = await connection.getMinimumBalanceForRentExemption(accountLen);

    const owner = Keypair.generate();
    const accountKeypair = Keypair.generate();
    const account = accountKeypair.publicKey;
    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: account,
            space: accountLen,
            lamports,
            programId: TOKEN_2022_PROGRAM_ID,
        }),
        createInitializeImmutableOwnerInstruction(account, TOKEN_2022_PROGRAM_ID),
        createInitializeAccountInstruction(account, mint, owner.publicKey, TOKEN_2022_PROGRAM_ID),
    );
    await sendAndConfirmTransaction(connection, transaction, [payer, accountKeypair], undefined);

    // create associated token account
    await createAccount(connection, payer, mint, owner.publicKey, undefined, undefined, TOKEN_2022_PROGRAM_ID);
})();
