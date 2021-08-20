import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { PublicKey, SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import { struct, u8 } from 'buffer-layout';
import { LENDING_PROGRAM_ID } from '../constants';
import { ReserveConfig, ReserveConfigLayout } from '../state';
import { u64 } from '../util';
import { LendingInstruction } from './instruction';

interface Data {
    instruction: number;
    liquidityAmount: bigint;
    config: ReserveConfig;
}

const DataLayout = struct<Data>([u8('instruction'), u64('liquidityAmount'), ReserveConfigLayout]);

export const initReserveInstruction = (
    liquidityAmount: number | bigint,
    config: ReserveConfig,
    sourceLiquidity: PublicKey,
    destinationCollateral: PublicKey,
    reserve: PublicKey,
    liquidityMint: PublicKey,
    liquiditySupply: PublicKey,
    liquidityFeeReceiver: PublicKey,
    pythProduct: PublicKey,
    pythPrice: PublicKey,
    collateralMint: PublicKey,
    collateralSupply: PublicKey,
    lendingMarket: PublicKey,
    lendingMarketAuthority: PublicKey,
    lendingMarketOwner: PublicKey,
    transferAuthority: PublicKey
): TransactionInstruction => {
    const data = Buffer.alloc(DataLayout.span);
    DataLayout.encode(
        {
            instruction: LendingInstruction.InitReserve,
            liquidityAmount: BigInt(liquidityAmount),
            config,
        },
        data
    );

    const keys = [
        { pubkey: sourceLiquidity, isSigner: false, isWritable: true },
        { pubkey: destinationCollateral, isSigner: false, isWritable: true },
        { pubkey: reserve, isSigner: false, isWritable: true },
        { pubkey: liquidityMint, isSigner: false, isWritable: false },
        { pubkey: liquiditySupply, isSigner: false, isWritable: true },
        { pubkey: liquidityFeeReceiver, isSigner: false, isWritable: true },
        { pubkey: collateralMint, isSigner: false, isWritable: true },
        { pubkey: collateralSupply, isSigner: false, isWritable: true },
        { pubkey: pythProduct, isSigner: false, isWritable: false },
        { pubkey: pythPrice, isSigner: false, isWritable: false },
        { pubkey: lendingMarket, isSigner: false, isWritable: true },
        { pubkey: lendingMarketAuthority, isSigner: false, isWritable: false },
        { pubkey: lendingMarketOwner, isSigner: true, isWritable: false },
        { pubkey: transferAuthority, isSigner: true, isWritable: false },
        { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ];

    return new TransactionInstruction({
        keys,
        programId: LENDING_PROGRAM_ID,
        data,
    });
};
