import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { PublicKey, SYSVAR_RENT_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import { blob, struct, u8 } from 'buffer-layout';
import { LendingInstruction } from './instruction';
import { LENDING_PROGRAM_ID, ORACLE_PROGRAM_ID } from '../constants';
import { publicKey } from '../util';

/// 0
/// Initializes a new lending market.
///
/// Accounts expected by this instruction:
///
///   0. `[writable]` Lending market account - uninitialized.
///   1. `[]` Rent sysvar.
///   2. `[]` Token program id.
///   3. `[]` Oracle program id.
///
/// InitLendingMarket {
///     /// Owner authority which can add new reserves
///     owner: Pubkey,
///     /// Currency market prices are quoted in
///     /// e.g. "USD" null padded (`*b"USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"`) or SPL token mint pubkey
///     quote_currency: [u8; 32],
/// },
export const initLendingMarketInstruction = (
    owner: PublicKey,
    quoteCurrency: Buffer,
    lendingMarket: PublicKey
): TransactionInstruction => {
    const dataLayout = struct([u8('instruction'), publicKey('owner'), blob(32, 'quoteCurrency')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: LendingInstruction.InitLendingMarket,
            owner,
            quoteCurrency,
        },
        data
    );

    const keys = [
        { pubkey: lendingMarket, isSigner: false, isWritable: true },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: ORACLE_PROGRAM_ID, isSigner: false, isWritable: false },
    ];

    return new TransactionInstruction({
        keys,
        programId: LENDING_PROGRAM_ID,
        data,
    });
};
