import { struct, u32, u8 } from '@solana/buffer-layout';
import { publicKey, u64 } from '@solana/buffer-layout-utils';
import { Commitment, Connection, PublicKey } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, TokenError } from '../constants';

/** @TODO: docs */
export enum AccountState {
    Uninitialized,
    Initialized,
    Frozen,
}

/** Information about an account */
export interface Account {
    /** The address of this account */
    address: PublicKey;
    /** The mint associated with this account */
    mint: PublicKey;
    /** Owner of this account */
    owner: PublicKey;
    /** Amount of tokens this account holds */
    amount: bigint;
    /** The delegate for this account */
    delegate: PublicKey | null;
    /** The amount of tokens the delegate authorized to the delegate */
    delegatedAmount: bigint;
    /** Is this account initialized */
    isInitialized: boolean;
    /** Is this account frozen */
    isFrozen: boolean;
    /** Is this a native token account */
    isNative: boolean;
    /**
     * If this account is a native token, it must be rent-exempt. This value logs the rent-exempt reserve which must
     * remain in the balance until the account is closed.
     */
    rentExemptReserve: bigint | null;
    /** Optional authority to close the account */
    closeAuthority: PublicKey | null;
}

/** @TODO: docs */
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

/** @TODO: docs */
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

/** @TODO: docs */
export const ACCOUNT_LEN = AccountLayout.span;

/** Get the minimum lamport balance for an Account to be rent exempt
 *
 * @param connection @TODO: docs
 * @param commitment @TODO: docs
 *
 * @return amount of lamports required
 */
export async function getMinimumBalanceForRentExemptAccount(
    connection: Connection,
    commitment?: Commitment
): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(ACCOUNT_LEN, commitment);
}

/**
 * Retrieve account information
 *
 * @param account Public key of the account
 *
 * @TODO: docs
 */
export async function getAccountInfo(
    connection: Connection,
    account: PublicKey,
    commitment?: Commitment,
    programId = TOKEN_PROGRAM_ID
): Promise<Account> {
    const info = await connection.getAccountInfo(account, commitment);
    if (!info) throw new Error(TokenError.ACCOUNT_NOT_FOUND);
    if (!info.owner.equals(programId)) throw new Error(TokenError.INVALID_ACCOUNT_OWNER);
    if (info.data.length != ACCOUNT_LEN) throw new Error(TokenError.INVALID_ACCOUNT_SIZE);

    const accountInfo = AccountLayout.decode(Buffer.from(info.data));
    // @FIXME
    accountInfo.address = account;

    if (!accountInfo.delegateOption) {
        accountInfo.delegate = null;
        accountInfo.delegatedAmount = BigInt(0);
    }

    accountInfo.isInitialized = accountInfo.state !== AccountState.Initialized;
    accountInfo.isFrozen = accountInfo.state === AccountState.Frozen;

    if (accountInfo.isNativeOption) {
        accountInfo.rentExemptReserve = accountInfo.isNative;
        accountInfo.isNative = true;
    } else {
        accountInfo.rentExemptReserve = null;
        accountInfo.isNative = false;
    }

    if (!accountInfo.closeAuthorityOption) {
        accountInfo.closeAuthority = null;
    }

    return accountInfo;
}
