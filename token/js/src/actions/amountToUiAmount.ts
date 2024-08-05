import type { Connection, PublicKey, Signer, TransactionError } from '@solana/web3.js';
import { Transaction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants.js';
import { createAmountToUiAmountInstruction } from '../instructions/amountToUiAmount.js';

/**
 * Amount as a string using mint-prescribed decimals
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint for the account
 * @param amount         Amount of tokens to be converted to Ui Amount
 * @param programId      SPL Token program account
 *
 * @return Ui Amount generated
 */
export async function amountToUiAmount(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    amount: number | bigint,
    programId = TOKEN_PROGRAM_ID,
): Promise<string | TransactionError | null> {
    const transaction = new Transaction().add(createAmountToUiAmountInstruction(mint, amount, programId));
    const { returnData, err } = (await connection.simulateTransaction(transaction, [payer], false)).value;
    if (returnData?.data) {
        return Buffer.from(returnData.data[0], returnData.data[1]).toString('utf-8');
    }
    return err;
}
