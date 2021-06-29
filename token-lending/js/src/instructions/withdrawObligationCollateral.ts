import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { PublicKey, SYSVAR_CLOCK_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import BN from 'bn.js';
import { struct, u8 } from 'buffer-layout';
import { LendingInstruction } from './instruction';
import { LENDING_PROGRAM_ID } from '../constants';
import { u64 } from '../util';

/// 9
/// Withdraw collateral from an obligation. Requires a refreshed obligation and reserve.
///
/// Accounts expected by this instruction:
///
///   0. `[writable]` Source withdraw reserve collateral supply SPL Token account.
///   1. `[writable]` Destination collateral token account.
///                     Minted by withdraw reserve collateral mint.
///   2. `[]` Withdraw reserve account - refreshed.
///   3. `[writable]` Obligation account - refreshed.
///   4. `[]` Lending market account.
///   5. `[]` Derived lending market authority.
///   6. `[signer]` Obligation owner.
///   7. `[]` Clock sysvar.
///   8. `[]` Token program id.
///
/// WithdrawObligationCollateral {
///     /// Amount of collateral tokens to withdraw - u64::MAX for up to 100% of deposited amount
///     collateral_amount: u64,
/// },
export const withdrawObligationCollateralInstruction = (
    collateralAmount: number | BN,
    sourceCollateral: PublicKey,
    destinationCollateral: PublicKey,
    withdrawReserve: PublicKey,
    obligation: PublicKey,
    lendingMarket: PublicKey,
    lendingMarketAuthority: PublicKey,
    obligationOwner: PublicKey
): TransactionInstruction => {
    const dataLayout = struct([u8('instruction'), u64('collateralAmount')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: LendingInstruction.WithdrawObligationCollateral,
            collateralAmount: new BN(collateralAmount),
        },
        data
    );

    const keys = [
        { pubkey: sourceCollateral, isSigner: false, isWritable: true },
        { pubkey: destinationCollateral, isSigner: false, isWritable: true },
        { pubkey: withdrawReserve, isSigner: false, isWritable: false },
        { pubkey: obligation, isSigner: false, isWritable: true },
        { pubkey: lendingMarket, isSigner: false, isWritable: false },
        { pubkey: lendingMarketAuthority, isSigner: false, isWritable: false },
        { pubkey: obligationOwner, isSigner: true, isWritable: false },
        { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ];

    return new TransactionInstruction({
        keys,
        programId: LENDING_PROGRAM_ID,
        data,
    });
};
