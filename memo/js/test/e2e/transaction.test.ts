import { createMemoInstruction } from '../../src/index';
import { Connection, Keypair, Transaction, LAMPORTS_PER_SOL, sendAndConfirmTransaction } from '@solana/web3.js';

test('transaction: live', async () => {
    const url = 'http://127.0.0.1:8899';
    const connection = new Connection(url, 'confirmed');
    await connection.getVersion();
    const signer = new Keypair(); // also fee-payer

    const airdropSignature = await connection.requestAirdrop(signer.publicKey, LAMPORTS_PER_SOL / 10);
    await connection.confirmTransaction(airdropSignature, 'confirmed');

    const memoTx = new Transaction().add(createMemoInstruction('this is a test memo', [signer.publicKey]));
    await sendAndConfirmTransaction(connection, memoTx, [signer], {
        preflightCommitment: 'confirmed',
    });
});
