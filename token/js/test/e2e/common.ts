import { Keypair, Connection, Signer } from '@solana/web3.js';

export async function newAccountWithLamports(connection: Connection, lamports = 1000000): Promise<Signer> {
    const account = Keypair.generate();
    const signature = await connection.requestAirdrop(account.publicKey, lamports);
    await connection.confirmTransaction(signature);
    return account;
}

export async function getConnection(): Promise<Connection> {
    const url = 'http://localhost:8899';
    const connection = new Connection(url, 'confirmed');
    await connection.getVersion();
    return connection;
}
