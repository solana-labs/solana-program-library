import {Account, Connection} from "@solana/web3.js";

export async function newAccountWithLamports(
    connection: Connection,
    lamports: number = 1000000,
): Promise<Account> {
    const account = new Account();

    let retries = 30;
    // console.log("new account is ", account);
    await connection.requestAirdrop(account.publicKey, lamports);
    for (; ;) {
        // console.log('round', retries)
        await sleep(500);
        if (lamports == (await connection.getBalance(account.publicKey))) {
            return account;
        }
        if (--retries <= 0) {
            break;
        }
    }
    throw new Error(`Airdrop of ${lamports} failed`);
}

export function sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
}

