import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { PublicKey, SYSVAR_CLOCK_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import BN from 'bn.js';
import { struct, u8 } from 'buffer-layout';
import { LendingInstruction } from './instruction';
import { LENDING_PROGRAM_ID } from '../constants';
import { u64 } from '../util';

/// 8
/// Deposit collateral to an obligation. Requires a refreshed reserve.
///
/// Accounts expected by this instruction:
///
///   0. `[writable]` Source collateral token account.
///                     Minted by deposit reserve collateral mint.
///                     $authority can transfer $collateral_amount.
///   1. `[writable]` Destination deposit reserve collateral supply SPL Token account.
///   2. `[]` Deposit reserve account - refreshed.
///   3. `[writable]` Obligation account.
///   4. `[]` Lending market account.
///   5. `[]` Derived lending market authority.
///   6. `[signer]` Obligation owner.
///   7. `[signer]` User transfer authority ($authority).
///   8. `[]` Clock sysvar.
///   9. `[]` Token program id.
///
/// DepositObligationCollateral {
///     /// Amount of collateral tokens to deposit
///     collateral_amount: u64,
/// },
export const depositObligationCollateralInstruction = (
    collateralAmount: number | BN,
    sourceCollateral: PublicKey,
    destinationCollateral: PublicKey,
    depositReserve: PublicKey,
    obligation: PublicKey,
    lendingMarket: PublicKey,
    lendingMarketAuthority: PublicKey,
    obligationOwner: PublicKey,
    transferAuthority: PublicKey
): TransactionInstruction => {
    const dataLayout = struct([u8('instruction'), u64('collateralAmount')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: LendingInstruction.DepositObligationCollateral,
            collateralAmount: new BN(collateralAmount),
        },
        data
    );

    const keys = [
        { pubkey: sourceCollateral, isSigner: false, isWritable: true },
        { pubkey: destinationCollateral, isSigner: false, isWritable: true },
        { pubkey: depositReserve, isSigner: false, isWritable: false },
        { pubkey: obligation, isSigner: false, isWritable: true },
        { pubkey: lendingMarket, isSigner: false, isWritable: false },
        { pubkey: lendingMarketAuthority, isSigner: false, isWritable: false },
        { pubkey: obligationOwner, isSigner: true, isWritable: false },
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
