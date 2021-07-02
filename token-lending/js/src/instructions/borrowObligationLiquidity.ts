import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { PublicKey, SYSVAR_CLOCK_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import { struct, u8 } from 'buffer-layout';
import { LENDING_PROGRAM_ID } from '../constants';
import { u64 } from '../util';
import { LendingInstruction } from './instruction';

interface Data {
    instruction: number;
    liquidityAmount: bigint;
}

const DataLayout = struct<Data>([u8('instruction'), u64('liquidityAmount')]);

export const borrowObligationLiquidityInstruction = (
    liquidityAmount: number | bigint,
    sourceLiquidity: PublicKey,
    destinationLiquidity: PublicKey,
    borrowReserve: PublicKey,
    borrowReserveLiquidityFeeReceiver: PublicKey,
    obligation: PublicKey,
    lendingMarket: PublicKey,
    lendingMarketAuthority: PublicKey,
    obligationOwner: PublicKey,
    hostFeeReceiver?: PublicKey
): TransactionInstruction => {
    const data = Buffer.alloc(DataLayout.span);
    DataLayout.encode(
        {
            instruction: LendingInstruction.BorrowObligationLiquidity,
            liquidityAmount: BigInt(liquidityAmount),
        },
        data
    );

    const keys = [
        { pubkey: sourceLiquidity, isSigner: false, isWritable: true },
        { pubkey: destinationLiquidity, isSigner: false, isWritable: true },
        { pubkey: borrowReserve, isSigner: false, isWritable: true },
        {
            pubkey: borrowReserveLiquidityFeeReceiver,
            isSigner: false,
            isWritable: true,
        },
        { pubkey: obligation, isSigner: false, isWritable: true },
        { pubkey: lendingMarket, isSigner: false, isWritable: false },
        { pubkey: lendingMarketAuthority, isSigner: false, isWritable: false },
        { pubkey: obligationOwner, isSigner: true, isWritable: false },
        { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ];

    if (hostFeeReceiver) {
        keys.push({ pubkey: hostFeeReceiver, isSigner: false, isWritable: true });
    }

    return new TransactionInstruction({
        keys,
        programId: LENDING_PROGRAM_ID,
        data,
    });
};
