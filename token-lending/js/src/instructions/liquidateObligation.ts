import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { PublicKey, SYSVAR_CLOCK_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import BN from 'bn.js';
import { struct, u8 } from 'buffer-layout';
import { LENDING_PROGRAM_ID } from '../constants';
import { u64 } from '../util';
import { LendingInstruction } from './instruction';

export const liquidateObligationInstruction = (
    liquidityAmount: number | BN,
    sourceLiquidity: PublicKey,
    destinationCollateral: PublicKey,
    repayReserve: PublicKey,
    repayReserveLiquiditySupply: PublicKey,
    withdrawReserve: PublicKey,
    withdrawReserveCollateralSupply: PublicKey,
    obligation: PublicKey,
    lendingMarket: PublicKey,
    lendingMarketAuthority: PublicKey,
    transferAuthority: PublicKey
): TransactionInstruction => {
    const dataLayout = struct([u8('instruction'), u64('liquidityAmount')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: LendingInstruction.LiquidateObligation,
            liquidityAmount: new BN(liquidityAmount),
        },
        data
    );

    const keys = [
        { pubkey: sourceLiquidity, isSigner: false, isWritable: true },
        { pubkey: destinationCollateral, isSigner: false, isWritable: true },
        { pubkey: repayReserve, isSigner: false, isWritable: true },
        { pubkey: repayReserveLiquiditySupply, isSigner: false, isWritable: true },
        { pubkey: withdrawReserve, isSigner: false, isWritable: false },
        {
            pubkey: withdrawReserveCollateralSupply,
            isSigner: false,
            isWritable: true,
        },
        { pubkey: obligation, isSigner: false, isWritable: true },
        { pubkey: lendingMarket, isSigner: false, isWritable: false },
        { pubkey: lendingMarketAuthority, isSigner: false, isWritable: false },
        { pubkey: transferAuthority, isSigner: true, isWritable: false },
        { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ];

    return new TransactionInstruction({
        keys,
        programId: LENDING_PROGRAM_ID,
        data,
    });
};
