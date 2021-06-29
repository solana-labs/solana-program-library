import { PublicKey, SYSVAR_CLOCK_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import { struct, u8 } from 'buffer-layout';
import { LendingInstruction } from './instruction';
import { LENDING_PROGRAM_ID } from '../constants';

/// 3
/// Accrue interest and update market price of liquidity on a reserve.
///
/// Accounts expected by this instruction:
///
///   0. `[writable]` Reserve account.
///   1. `[]` Reserve liquidity oracle account.
///             Must be the Pyth price account specified at InitReserve.
///   2. `[]` Clock sysvar.
export const refreshReserveInstruction = (reserve: PublicKey, oracle: PublicKey): TransactionInstruction => {
    const dataLayout = struct([u8('instruction')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: LendingInstruction.RefreshReserve }, data);

    const keys = [
        { pubkey: reserve, isSigner: false, isWritable: true },
        { pubkey: oracle, isSigner: false, isWritable: false },
        { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    ];

    return new TransactionInstruction({
        keys,
        programId: LENDING_PROGRAM_ID,
        data,
    });
};
