import { PublicKey, SYSVAR_CLOCK_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import { struct, u8 } from 'buffer-layout';
import { LENDING_PROGRAM_ID } from '../constants';
import { LendingInstruction } from './instruction';

interface Data {
    instruction: number;
}

const DataLayout = struct<Data>([u8('instruction')]);

export const refreshObligationInstruction = (
    obligation: PublicKey,
    depositReserves: PublicKey[],
    borrowReserves: PublicKey[]
): TransactionInstruction => {
    const data = Buffer.alloc(DataLayout.span);
    DataLayout.encode({ instruction: LendingInstruction.RefreshObligation }, data);

    const keys = [
        { pubkey: obligation, isSigner: false, isWritable: true },
        { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    ];

    for (const depositReserve of depositReserves) {
        keys.push({ pubkey: depositReserve, isSigner: false, isWritable: false });
    }

    for (const borrowReserve of borrowReserves) {
        keys.push({ pubkey: borrowReserve, isSigner: false, isWritable: false });
    }

    return new TransactionInstruction({
        keys,
        programId: LENDING_PROGRAM_ID,
        data,
    });
};
