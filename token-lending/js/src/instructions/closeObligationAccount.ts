import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { struct, u8 } from '@solana/buffer-layout';
import { LENDING_PROGRAM_ID } from '../constants';
import { LendingInstruction } from './instruction';

interface Data {
    instruction: number;
}

const DataLayout = struct<Data>([u8('instruction')])

export const closeObligationAccountInstruction = (
    obligation: PublicKey,
    obligationOwner: PublicKey,
    destination: PublicKey,
    reserve: PublicKey,
    lendingMarketAuthority: PublicKey
): TransactionInstruction => {
    const data = Buffer.alloc(DataLayout.span);
    DataLayout.encode({ instruction: LendingInstruction.CloseObligationAccount }, data);
    const keys = [
        { pubkey: obligation, isSigner: false, isWritable: true },
        { pubkey: obligationOwner, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        { pubkey: reserve, isSigner: false, isWritable: false },
        { pubkey: lendingMarketAuthority, isSigner: true, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ] 
    
    return new TransactionInstruction ({
        keys,
        programId: LENDING_PROGRAM_ID,
        data,
    });
};