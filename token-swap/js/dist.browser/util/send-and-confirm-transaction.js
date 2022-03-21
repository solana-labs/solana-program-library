import { sendAndConfirmTransaction as realSendAndConfirmTransaction } from '@solana/web3.js';
export function sendAndConfirmTransaction(title, connection, transaction, ...signers) {
    return realSendAndConfirmTransaction(connection, transaction, signers, {
        skipPreflight: false,
        commitment: 'recent',
        preflightCommitment: 'recent',
    });
}
