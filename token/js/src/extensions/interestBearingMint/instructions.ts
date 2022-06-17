import { struct, s16, u8 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_2022_PROGRAM_ID } from '../../constants';
import { TokenInstruction } from '../../instructions';

export enum InterestBearingMintInstruction {
    Initialize = 0,
    UpdateRate = 1,
}

export interface InterestBearingMintInitializeInstructionData {
    instruction: TokenInstruction.InterestBearingMintExtension;
    interestBearingMintInstruction: InterestBearingMintInstruction.Initialize;
    rateAuthority: PublicKey;
    rate: number;
}

export interface InterestBearingMintUpdateRateInstructionData {
    instruction: TokenInstruction.InterestBearingMintExtension;
    interestBearingMintInstruction: InterestBearingMintInstruction.UpdateRate;
    rate: number;
}

export const interestBearingMintInitializeInstructionData = struct<InterestBearingMintInitializeInstructionData>([
    u8('instruction'),
    u8('interestBearingMintInstruction'),
    publicKey('rateAuthority'),
    s16('rate'),
]);

export const interestBearingMintUpdateRateInstructionData = struct<InterestBearingMintUpdateRateInstructionData>([
    u8('instruction'),
    u8('interestBearingMintInstruction'),
    s16('rate'),
]);

export const INTEREST_BEARING_MINT_INITIALIZE_SIZE = interestBearingMintInitializeInstructionData.span;
export const INTEREST_BEARING_MINT_UPDATE_RATE_SIZE = interestBearingMintUpdateRateInstructionData.span;

/**
 * Construct an InitializeInterestBearingMint instruction
 *
 * @param mint           Mint to initialize
 * @param rateAuthority  The public key for the account that can update the rate
 * @param rate           The initial interest rate
 * @param programId      SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createInitializeInterestBearingMintInstruction(
    mint: PublicKey,
    rateAuthority: PublicKey,
    rate: number,
    programId = TOKEN_2022_PROGRAM_ID
) {
    const keys = [{ pubkey: mint, isSigner: false, isWritable: true }];
    const data = Buffer.alloc(interestBearingMintInitializeInstructionData.span);
    interestBearingMintInitializeInstructionData.encode(
        {
            instruction: TokenInstruction.InterestBearingMintExtension,
            interestBearingMintInstruction: InterestBearingMintInstruction.Initialize,
            rateAuthority,
            rate,
        },
        data
    );
    return new TransactionInstruction({ keys, programId, data });
}

/**
 * Construct an UpdateRateInterestBearingMint instruction
 *
 * @param mint           Mint to initialize
 * @param rate           The updated interest rate
 * @param programId      SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createUpdateRateInterestBearingMintInstruction(
    mint: PublicKey,
    rate: number,
    programId = TOKEN_2022_PROGRAM_ID
) {
    const keys = [{ pubkey: mint, isSigner: false, isWritable: true }];
    const data = Buffer.alloc(interestBearingMintInitializeInstructionData.span);
    interestBearingMintUpdateRateInstructionData.encode(
        {
            instruction: TokenInstruction.InterestBearingMintExtension,
            interestBearingMintInstruction: InterestBearingMintInstruction.UpdateRate,
            rate,
        },
        data
    );
    return new TransactionInstruction({ keys, programId, data });
}
