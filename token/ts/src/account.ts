import { struct, u32, u8 } from '@solana/buffer-layout';
import { publicKey, u64 } from '@solana/buffer-layout-utils';
import { PublicKey } from '@solana/web3.js';

export enum AccountState {
    Uninitialized,
    Initialized,
    Frozen,
}

/**
 * Information about an account
 */
export interface Account {
    /**
     * The address of this account
     */
    address: PublicKey;

    /**
     * The mint associated with this account
     */
    mint: PublicKey;

    /**
     * Owner of this account
     */
    owner: PublicKey;

    /**
     * Amount of tokens this account holds
     */
    amount: bigint;

    /**
     * The delegate for this account
     */
    delegate: PublicKey | null;

    /**
     * The amount of tokens the delegate authorized to the delegate
     */
    delegatedAmount: bigint;

    /**
     * Is this account initialized
     */
    isInitialized: boolean;

    /**
     * Is this account frozen
     */
    isFrozen: boolean;

    /**
     * Is this a native token account
     */
    isNative: boolean;

    /**
     * If this account is a native token, it must be rent-exempt. This
     * value logs the rent-exempt reserve which must remain in the balance
     * until the account is closed.
     */
    rentExemptReserve: bigint | null;

    /**
     * Optional authority to close the account
     */
    closeAuthority: PublicKey | null;
}

export interface RawAccount {
    mint: PublicKey;
    owner: PublicKey;
    amount: bigint;
    delegateOption: 1 | 0;
    delegate: PublicKey;
    state: AccountState;
    isNativeOption: 1 | 0;
    isNative: bigint;
    delegatedAmount: bigint;
    closeAuthorityOption: 1 | 0;
    closeAuthority: PublicKey;
}

export const AccountLayout = struct<RawAccount>([
    publicKey('mint'),
    publicKey('owner'),
    u64('amount'),
    u32('delegateOption'),
    publicKey('delegate'),
    u8('state'),
    u32('isNativeOption'),
    u64('isNative'),
    u64('delegatedAmount'),
    u32('closeAuthorityOption'),
    publicKey('closeAuthority'),
]);

export const ACCOUNT_LEN = AccountLayout.span;
