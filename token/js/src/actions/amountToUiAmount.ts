import {
    Connection,
    PublicKey,
    Signer,
    Transaction,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createAmountToUiAmountInstruction } from '../instructions/index';

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
    programId = TOKEN_PROGRAM_ID
): Promise<string> {
    const transaction = new Transaction().add(
        createAmountToUiAmountInstruction(mint,amount,programId)
    );
    try{
        const {returnData} = (await connection.simulateTransaction(transaction, [payer] , false)).value;
        if(returnData?.data){
            return Buffer.from(returnData.data[0],returnData.data[1]).toString();
        }else{
            throw new Error("Amount Cannot be converted to Ui Amount !");
        }
    }catch(error:unknown){
       throw error;
    }
}