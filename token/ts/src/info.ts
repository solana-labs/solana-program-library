import { Commitment, Connection, PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { Account, ACCOUNT_LEN, AccountLayout, AccountState } from './account';
import { TOKEN_PROGRAM_ID } from './constants';
import { Mint, MINT_LEN, MintLayout } from './mint';
import { Multisig, MULTISIG_LEN, MultisigLayout } from './multisig';

export const FAILED_TO_FIND_ACCOUNT = 'Failed to find account';
export const INVALID_ACCOUNT_OWNER = 'Invalid account owner';

/**
 * Retrieve mint information
 */
export async function getMintInfo(
    connection: Connection,
    address: PublicKey,
    programId = TOKEN_PROGRAM_ID
): Promise<Mint> {
    const info = await connection.getAccountInfo(address);
    if (info === null) {
        throw new Error('Failed to find mint account');
    }
    if (!info.owner.equals(programId)) {
        throw new Error(`Invalid mint owner: ${JSON.stringify(info.owner)}`);
    }
    if (info.data.length != MINT_LEN) {
        throw new Error(`Invalid mint size`);
    }

    const mintInfo = MintLayout.decode(Buffer.from(info.data));

    if (!mintInfo.mintAuthorityOption) {
        mintInfo.mintAuthority = null;
    }

    if (!mintInfo.freezeAuthorityOption) {
        mintInfo.freezeAuthority = null;
    }

    return mintInfo;
}

/**
 * Retrieve account information
 *
 * @param account Public key of the account
 */
export async function getAccountInfo(
    connection: Connection,
    mint: PublicKey,
    account: PublicKey,
    commitment?: Commitment,
    programId = TOKEN_PROGRAM_ID
): Promise<Account> {
    const info = await connection.getAccountInfo(account, commitment);
    if (info === null) {
        throw new Error(FAILED_TO_FIND_ACCOUNT);
    }
    if (!info.owner.equals(programId)) {
        throw new Error(INVALID_ACCOUNT_OWNER);
    }
    if (info.data.length != ACCOUNT_LEN) {
        throw new Error(`Invalid account size`);
    }

    const accountInfo = AccountLayout.decode(Buffer.from(info.data));
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

    if (!accountInfo.mint.equals(mint)) {
        throw new Error(`Invalid account mint: ${JSON.stringify(accountInfo.mint)} !== ${JSON.stringify(mint)}`);
    }

    return accountInfo;
}

/**
 * Retrieve Multisig information
 *
 * @param multisig Public key of the account
 */
export async function getMultisigInfo(
    connection: Connection,
    multisig: PublicKey,
    programId = TOKEN_PROGRAM_ID
): Promise<Multisig> {
    const info = await connection.getAccountInfo(multisig);
    if (info === null) {
        throw new Error('Failed to find multisig');
    }
    if (!info.owner.equals(programId)) {
        throw new Error(`Invalid multisig owner`);
    }
    if (info.data.length != MULTISIG_LEN) {
        throw new Error(`Invalid multisig size`);
    }

    return MultisigLayout.decode(Buffer.from(info.data));
}
