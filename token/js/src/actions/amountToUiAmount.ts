import type { Connection, PublicKey, Signer, TransactionError } from '@solana/web3.js';
import Decimal from 'decimal.js';
import { Transaction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants.js';
import { createAmountToUiAmountInstruction } from '../instructions/amountToUiAmount.js';
import { getMint } from '../state/mint.js';
import { getInterestBearingMintConfigState } from '../extensions/interestBearingMint/state.js';

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

const ONE_IN_BASIS_POINTS = 10000;
const SECONDS_PER_YEAR = 60 * 60 * 24 * 365.24;

/**
 * Convert amount to UiAmount for a mint with interest bearing extension without simulating a transaction
 * This implements the same logic as the CPI instruction available in /token/program-2022/src/extension/interest_bearing_mint/mod.rs
 * 
 * @param amount                   Amount of tokens to be converted
 * @param decimals                 Number of decimals of the mint
 * @param currentTimestamp         Current timestamp in seconds
 * @param lastUpdateTimestamp      Last time the interest rate was updated in seconds
 * @param initializationTimestamp  Time the interest bearing extension was initialized in seconds
 * @param preUpdateAverageRate     Interest rate in basis points (1 basis point = 0.01%) before last update
 * @param currentRate              Current interest rate in basis points
 * 
 * @return Amount scaled by accrued interest as a string with appropriate decimal places
 */

export function amountToUiAmountWithoutSimulation(
    amount: string,
    decimals: number,
    currentTimestamp: number, // in seconds
    lastUpdateTimestamp: number,
    initializationTimestamp: number,
    preUpdateAverageRate: number,
    currentRate: number,
): string {
    Decimal.set({ toExpPos: 24, toExpNeg: -24 })

    // Calculate pre-update exponent
    const preUpdateTimespan = new Decimal(lastUpdateTimestamp).minus(initializationTimestamp);
    const preUpdateNumerator = new Decimal(preUpdateAverageRate).times(preUpdateTimespan);
    const preUpdateExponent = preUpdateNumerator.div(new Decimal(SECONDS_PER_YEAR).times(ONE_IN_BASIS_POINTS));
    const preUpdateExp = preUpdateExponent.exp();

    // Calculate post-update exponent
    const postUpdateTimespan = new Decimal(currentTimestamp).minus(lastUpdateTimestamp);
    const postUpdateNumerator = new Decimal(currentRate).times(postUpdateTimespan);
    const postUpdateExponent = postUpdateNumerator.div(new Decimal(SECONDS_PER_YEAR).times(ONE_IN_BASIS_POINTS));
    const postUpdateExp = postUpdateExponent.exp();

    // Calculate total scale
    const totalScale = preUpdateExp.times(postUpdateExp);

    // Calculate scaled amount with interest rounded down to the nearest unit
    const decimalsFactor = new Decimal(10).pow(decimals);
    const scaledAmountWithInterest = new Decimal(amount).times(totalScale).div(decimalsFactor).toDecimalPlaces(decimals, Decimal.ROUND_DOWN);

    return scaledAmountWithInterest.toString();
}

/**
 * Convert amount to UiAmount for a mint with interest bearing extension without simulating a transaction
 * This implements the same logic as the CPI instruction available in /token/program-2022/src/extension/interest_bearing_mint/mod.rs
 *
 * @param connection     Connection to use
 * @param mint           Mint for the account
 * @param amount         Amount of tokens to be converted to Ui Amount
 * @param programId      SPL Token program account (default: TOKEN_PROGRAM_ID)
 *
 * @return Ui Amount generated
 */
export async function amountToUiAmountForMintWithoutSimulation(
    connection: Connection,
    mint: PublicKey,
    amount: string,
    programId = TOKEN_PROGRAM_ID,
): Promise<string> {
    Decimal.set({ toExpPos: 24, toExpNeg: -24 })
    const mintInfo = await getMint(connection, mint, 'confirmed', programId);
    const amountDecimal = new Decimal(amount.toString());
    const decimalsFactor = new Decimal(10).pow(mintInfo.decimals);

    if (programId.equals(TOKEN_PROGRAM_ID)) {
        console.log('amountDecimal', amountDecimal.toString(), 'mintInfo', mintInfo);
        return amountDecimal.div(decimalsFactor).toString();
    }

    const interestBearingMintConfigState = getInterestBearingMintConfigState(mintInfo);
    if (!interestBearingMintConfigState) {
        return amountDecimal.div(decimalsFactor).toString();
    }

    const currentTime = Math.floor(Date.now() / 1000); // Convert to seconds
    return amountToUiAmountWithoutSimulation(
        amount,
        mintInfo.decimals,
        currentTime,
        interestBearingMintConfigState.lastUpdateTimestamp,
        interestBearingMintConfigState.initializationTimestamp,
        interestBearingMintConfigState.preUpdateAverageRate,
        interestBearingMintConfigState.currentRate
    );
}

/**
 * Convert an amount with interest back to the original amount without interest
 * This implements the same logic as the CPI instruction available in /token/program-2022/src/extension/interest_bearing_mint/mod.rs
 * 
 * @param uiAmount                  UI Amount (principle plus continuously compounding interest) to be converted back to original principle
 * @param decimals                  Number of decimals for the mint
 * @param currentTimestamp          Current timestamp in seconds
 * @param lastUpdateTimestamp       Last time the interest rate was updated in seconds
 * @param initializationTimestamp   Time the interest bearing extension was initialized in seconds
 * @param preUpdateAverageRate      Interest rate in basis points (hundredths of a percent) before the last update
 * @param currentRate              Current interest rate in basis points
 * 
 * @return Original amount (principle) without interest
 */

export function uiAmountToAmountWithoutSimulation(
    uiAmount: string,
    decimals: number,
    currentTimestamp: number, // in seconds
    lastUpdateTimestamp: number,
    initializationTimestamp: number,
    preUpdateAverageRate: number,
    currentRate: number,
): bigint {
    Decimal.set({ toExpPos: 24, toExpNeg: -24 })
    const uiAmountDecimal = new Decimal(uiAmount);
    const decimalsFactor = new Decimal(10).pow(decimals);
    const uiAmountScaled = uiAmountDecimal.mul(decimalsFactor);
   
    // Calculate pre-update exponent
    const preUpdateTimespan = new Decimal(lastUpdateTimestamp).minus(initializationTimestamp);
    const preUpdateNumerator = new Decimal(preUpdateAverageRate).times(preUpdateTimespan);
    const preUpdateExponent = preUpdateNumerator.div(new Decimal(SECONDS_PER_YEAR).times(ONE_IN_BASIS_POINTS));
    const preUpdateExp = preUpdateExponent.exp();

    // Calculate post-update exponent
    const postUpdateTimespan = new Decimal(currentTimestamp).minus(lastUpdateTimestamp);
    const postUpdateNumerator = new Decimal(currentRate).times(postUpdateTimespan);
    const postUpdateExponent = postUpdateNumerator.div(new Decimal(SECONDS_PER_YEAR).times(ONE_IN_BASIS_POINTS));
    const postUpdateExp = postUpdateExponent.exp();

    // Calculate total scale
    const totalScale = preUpdateExp.times(postUpdateExp);

    // Calculate original principle by dividing the UI amount (principle + interest) by the total scale
    const originalPrinciple = uiAmountScaled.div(totalScale);
    return BigInt(originalPrinciple.trunc().toString()); 
}

/**
 * Convert a UI amount with interest back to the original UI amount without interest
 * 
 * @param connection     Connection to use
 * @param mint           Mint to get decimals from
 * @param uiAmount       UI Amount (principle plus continuously compounding interest) to be converted back to original principle
 * @param programId      SPL Token program account (default: TOKEN_PROGRAM_ID)
 * 
 * @return Original UI Amount (principle) without interest
 */
export async function uiAmountToAmountForMintWithoutSimulation(
    connection: Connection,
    mint: PublicKey,
    uiAmount: string,
    programId = TOKEN_PROGRAM_ID,
): Promise<bigint> {
    Decimal.set({ toExpPos: 24, toExpNeg: -24 })
    const mintInfo = await getMint(connection, mint, 'confirmed', programId);
    const uiAmountScaled = new Decimal(uiAmount).mul(new Decimal(10).pow(mintInfo.decimals));

    if (programId.equals(TOKEN_PROGRAM_ID)) {
        return BigInt(uiAmountScaled.trunc().toString());
    }

    const interestBearingMintConfigState = getInterestBearingMintConfigState(mintInfo);
    if (!interestBearingMintConfigState) {
        return BigInt(uiAmountScaled.trunc().toString());
    }

    const currentTime = Math.floor(Date.now() / 1000); // Convert to seconds
    return uiAmountToAmountWithoutSimulation(
        uiAmount,
        mintInfo.decimals,
        currentTime,
        interestBearingMintConfigState.lastUpdateTimestamp,
        interestBearingMintConfigState.initializationTimestamp,
        interestBearingMintConfigState.preUpdateAverageRate,
        interestBearingMintConfigState.currentRate
    );
}

