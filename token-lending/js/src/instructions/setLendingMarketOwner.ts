import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { struct, u8 } from 'buffer-layout';
import { LENDING_PROGRAM_ID } from '../constants';
import { publicKey } from '../util';
import { LendingInstruction } from './instruction';

interface Data {
    instruction: number;
    newOwner: PublicKey;
}

export const setLendingMarketOwnerInstruction = (
    newOwner: PublicKey,
    lendingMarket: PublicKey,
    currentOwner: PublicKey
): TransactionInstruction => {
    const dataLayout = struct<Data>([u8('instruction'), publicKey('newOwner')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: LendingInstruction.SetLendingMarketOwner,
            newOwner,
        },
        data
    );

    const keys = [
        { pubkey: lendingMarket, isSigner: false, isWritable: true },
        { pubkey: currentOwner, isSigner: true, isWritable: false },
    ];

    return new TransactionInstruction({
        keys,
        programId: LENDING_PROGRAM_ID,
        data,
    });
};
