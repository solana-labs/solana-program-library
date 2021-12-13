import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { struct, u8 } from '@solana/buffer-layout';
import { LENDING_PROGRAM_ID } from '../constants';
import { publicKey } from '../util';
import { LendingInstruction } from './instruction';

interface Data {
    instruction: number;
    newOwner: PublicKey;
}

const DataLayout = struct([u8('instruction'), publicKey('newOwner')]);

export const setLendingMarketOwnerInstruction = (
    newOwner: PublicKey,
    lendingMarket: PublicKey,
    currentOwner: PublicKey
): TransactionInstruction => {
    const data = Buffer.alloc(DataLayout.span);
    DataLayout.encode(
        {
            instruction: LendingInstruction.SetLendingMarketOwner,
            newOwner,
        } as Data,
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
