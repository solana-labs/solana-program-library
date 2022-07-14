import {
    Connection,
    PublicKey,
    Signer,
    Transaction,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createUiAmountToAmountInstruction } from '../instructions/index';

/**
 * Amount as a string using mint-prescribed decimals
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint for the account
 * @param amount         Ui Amount of tokens to be converted to Amount
 * @param programId      SPL Token program account
 *
 * @return Ui Amount generated 
 */
export async function uiAmountToAmount(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    amount: string,
    programId = TOKEN_PROGRAM_ID
): Promise<number> {
    const transaction = new Transaction().add(
        createUiAmountToAmountInstruction(mint,amount,programId)
    );
    try{
        const {returnData,err} = (await connection.simulateTransaction(transaction, [payer] , false)).value;
        console.log(err)
        if(returnData?.data){
            const x = Buffer.from(returnData.data[0],returnData.data[1]);
            console.log(x);
            return 5245;
        }else{
            throw new Error("Amount Cannot be converted to Ui Amount !");
        }
    }catch(error:unknown){
       throw error;
    }
}