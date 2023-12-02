import type { AccountInfo, Connection, PublicKey } from '@solana/web3.js';

import { ACCOUNT_SIZE } from '../state/account.js';
import type { Mint } from '../state/mint.js';
import { MINT_SIZE, unpackMint } from '../state/mint.js';
import { MULTISIG_SIZE } from '../state/multisig.js';
import { ACCOUNT_TYPE_SIZE } from './accountType.js';
import { CPI_GUARD_SIZE } from './cpiGuard/index.js';
import { DEFAULT_ACCOUNT_STATE_SIZE } from './defaultAccountState/index.js';
import { IMMUTABLE_OWNER_SIZE } from './immutableOwner.js';
import { INTEREST_BEARING_MINT_CONFIG_STATE_SIZE } from './interestBearingMint/state.js';
import { MEMO_TRANSFER_SIZE } from './memoTransfer/index.js';
import { METADATA_POINTER_SIZE } from './metadataPointer/state.js';
import { MINT_CLOSE_AUTHORITY_SIZE } from './mintCloseAuthority.js';
import { NON_TRANSFERABLE_ACCOUNT_SIZE, NON_TRANSFERABLE_SIZE } from './nonTransferable.js';
import { PERMANENT_DELEGATE_SIZE } from './permanentDelegate.js';
import { TRANSFER_FEE_AMOUNT_SIZE, TRANSFER_FEE_CONFIG_SIZE } from './transferFee/index.js';
import { TRANSFER_HOOK_ACCOUNT_SIZE, TRANSFER_HOOK_SIZE } from './transferHook/index.js';
import { TOKEN_2022_PROGRAM_ID } from '../constants.js';
import { TokenAccountNotFoundError } from '../errors.js';

// Sequence from https://github.com/solana-labs/solana-program-library/blob/master/token/program-2022/src/extension/mod.rs#L903
export enum ExtensionType {
    Uninitialized,
    TransferFeeConfig,
    TransferFeeAmount,
    MintCloseAuthority,
    ConfidentialTransferMint,
    ConfidentialTransferAccount,
    DefaultAccountState,
    ImmutableOwner,
    MemoTransfer,
    NonTransferable,
    InterestBearingConfig,
    CpiGuard,
    PermanentDelegate,
    NonTransferableAccount,
    TransferHook,
    TransferHookAccount,
    // ConfidentialTransferFee, // Not implemented yet
    // ConfidentialTransferFeeAmount, // Not implemented yet
    MetadataPointer = 18, // Remove number once above extensions implemented
    TokenMetadata = 19, // Remove number once above extensions implemented
}

export const TYPE_SIZE = 2;
export const LENGTH_SIZE = 2;

// NOTE: All of these should eventually use their type's Span instead of these
// constants.  This is provided for at least creation to work.
export function getTypeLen(e: ExtensionType): number {
    switch (e) {
        case ExtensionType.Uninitialized:
            return 0;
        case ExtensionType.TransferFeeConfig:
            return TRANSFER_FEE_CONFIG_SIZE;
        case ExtensionType.TransferFeeAmount:
            return TRANSFER_FEE_AMOUNT_SIZE;
        case ExtensionType.MintCloseAuthority:
            return MINT_CLOSE_AUTHORITY_SIZE;
        case ExtensionType.ConfidentialTransferMint:
            return 97;
        case ExtensionType.ConfidentialTransferAccount:
            return 286;
        case ExtensionType.CpiGuard:
            return CPI_GUARD_SIZE;
        case ExtensionType.DefaultAccountState:
            return DEFAULT_ACCOUNT_STATE_SIZE;
        case ExtensionType.ImmutableOwner:
            return IMMUTABLE_OWNER_SIZE;
        case ExtensionType.MemoTransfer:
            return MEMO_TRANSFER_SIZE;
        case ExtensionType.MetadataPointer:
            return METADATA_POINTER_SIZE;
        case ExtensionType.NonTransferable:
            return NON_TRANSFERABLE_SIZE;
        case ExtensionType.InterestBearingConfig:
            return INTEREST_BEARING_MINT_CONFIG_STATE_SIZE;
        case ExtensionType.PermanentDelegate:
            return PERMANENT_DELEGATE_SIZE;
        case ExtensionType.NonTransferableAccount:
            return NON_TRANSFERABLE_ACCOUNT_SIZE;
        case ExtensionType.TransferHook:
            return TRANSFER_HOOK_SIZE;
        case ExtensionType.TransferHookAccount:
            return TRANSFER_HOOK_ACCOUNT_SIZE;
        case ExtensionType.TokenMetadata:
            throw Error(`Cannot get type length for variable extension type: ${e}`);
        default:
            throw Error(`Unknown extension type: ${e}`);
    }
}

export function isMintExtension(e: ExtensionType): boolean {
    switch (e) {
        case ExtensionType.TransferFeeConfig:
        case ExtensionType.MintCloseAuthority:
        case ExtensionType.ConfidentialTransferMint:
        case ExtensionType.DefaultAccountState:
        case ExtensionType.NonTransferable:
        case ExtensionType.InterestBearingConfig:
        case ExtensionType.PermanentDelegate:
        case ExtensionType.TransferHook:
        case ExtensionType.MetadataPointer:
        case ExtensionType.TokenMetadata:
            return true;
        case ExtensionType.Uninitialized:
        case ExtensionType.TransferFeeAmount:
        case ExtensionType.ConfidentialTransferAccount:
        case ExtensionType.ImmutableOwner:
        case ExtensionType.MemoTransfer:
        case ExtensionType.CpiGuard:
        case ExtensionType.NonTransferableAccount:
        case ExtensionType.TransferHookAccount:
            return false;
        default:
            throw Error(`Unknown extension type: ${e}`);
    }
}

export function isAccountExtension(e: ExtensionType): boolean {
    switch (e) {
        case ExtensionType.TransferFeeAmount:
        case ExtensionType.ConfidentialTransferAccount:
        case ExtensionType.ImmutableOwner:
        case ExtensionType.MemoTransfer:
        case ExtensionType.CpiGuard:
        case ExtensionType.NonTransferableAccount:
        case ExtensionType.TransferHookAccount:
            return true;
        case ExtensionType.Uninitialized:
        case ExtensionType.TransferFeeConfig:
        case ExtensionType.MintCloseAuthority:
        case ExtensionType.ConfidentialTransferMint:
        case ExtensionType.DefaultAccountState:
        case ExtensionType.NonTransferable:
        case ExtensionType.InterestBearingConfig:
        case ExtensionType.PermanentDelegate:
        case ExtensionType.TransferHook:
        case ExtensionType.MetadataPointer:
        case ExtensionType.TokenMetadata:
            return false;
        default:
            throw Error(`Unknown extension type: ${e}`);
    }
}

export function getAccountTypeOfMintType(e: ExtensionType): ExtensionType {
    switch (e) {
        case ExtensionType.TransferFeeConfig:
            return ExtensionType.TransferFeeAmount;
        case ExtensionType.ConfidentialTransferMint:
            return ExtensionType.ConfidentialTransferAccount;
        case ExtensionType.NonTransferable:
            return ExtensionType.NonTransferableAccount;
        case ExtensionType.TransferHook:
            return ExtensionType.TransferHookAccount;
        case ExtensionType.TransferFeeAmount:
        case ExtensionType.ConfidentialTransferAccount:
        case ExtensionType.CpiGuard:
        case ExtensionType.DefaultAccountState:
        case ExtensionType.ImmutableOwner:
        case ExtensionType.MemoTransfer:
        case ExtensionType.MintCloseAuthority:
        case ExtensionType.MetadataPointer:
        case ExtensionType.TokenMetadata:
        case ExtensionType.Uninitialized:
        case ExtensionType.InterestBearingConfig:
        case ExtensionType.PermanentDelegate:
        case ExtensionType.NonTransferableAccount:
        case ExtensionType.TransferHookAccount:
            return ExtensionType.Uninitialized;
    }
}

function getLen(extensionTypes: ExtensionType[], baseSize: number): number {
    if (extensionTypes.length === 0) {
        return baseSize;
    } else {
        const accountLength =
            ACCOUNT_SIZE +
            ACCOUNT_TYPE_SIZE +
            extensionTypes
                .filter((element, i) => i === extensionTypes.indexOf(element))
                .map((element) => getTypeLen(element) + TYPE_SIZE + LENGTH_SIZE)
                .reduce((a, b) => a + b);
        if (accountLength === MULTISIG_SIZE) {
            return accountLength + TYPE_SIZE;
        } else {
            return accountLength;
        }
    }
}

export function getMintLen(extensionTypes: ExtensionType[]): number {
    return getLen(extensionTypes, MINT_SIZE);
}

export function getAccountLen(extensionTypes: ExtensionType[]): number {
    return getLen(extensionTypes, ACCOUNT_SIZE);
}

export function getExtensionData(extension: ExtensionType, tlvData: Buffer): Buffer | null {
    let extensionTypeIndex = 0;
    while (extensionTypeIndex + TYPE_SIZE + LENGTH_SIZE <= tlvData.length) {
        const entryType = tlvData.readUInt16LE(extensionTypeIndex);
        const entryLength = tlvData.readUInt16LE(extensionTypeIndex + TYPE_SIZE);
        const typeIndex = extensionTypeIndex + TYPE_SIZE + LENGTH_SIZE;
        if (entryType == extension) {
            return tlvData.slice(typeIndex, typeIndex + entryLength);
        }
        extensionTypeIndex = typeIndex + entryLength;
    }
    return null;
}

export function getExtensionDataFromAccountInfo(
    address: PublicKey,
    accountInfo: AccountInfo<Buffer> | null,
    e: ExtensionType,
    programId = TOKEN_2022_PROGRAM_ID
): Buffer | null {
    const mint = unpackMint(address, accountInfo, programId);

    return getExtensionData(e, mint.tlvData);
}

export function getExtensionTypes(tlvData: Buffer): ExtensionType[] {
    const extensionTypes = [];
    let extensionTypeIndex = 0;
    while (extensionTypeIndex < tlvData.length) {
        const entryType = tlvData.readUInt16LE(extensionTypeIndex);
        extensionTypes.push(entryType);
        const entryLength = tlvData.readUInt16LE(extensionTypeIndex + TYPE_SIZE);
        extensionTypeIndex += TYPE_SIZE + LENGTH_SIZE + entryLength;
    }
    return extensionTypes;
}

export function getAccountLenForMint(mint: Mint): number {
    const extensionTypes = getExtensionTypes(mint.tlvData);
    const accountExtensions = extensionTypes.map(getAccountTypeOfMintType);
    return getAccountLen(accountExtensions);
}

export function getNewAccountLenForExtensionLen(
    info: AccountInfo<Buffer>,
    address: PublicKey,
    e: ExtensionType,
    extensionLen: number,
    programId = TOKEN_2022_PROGRAM_ID
): number {
    const data = getExtensionDataFromAccountInfo(address, info, e, programId);

    // 2 bytes discriminator, 2 bytes length, extensionLen
    const newExtensionLen = 2 + 2 + extensionLen;

    return info.data.length + newExtensionLen - (data?.length || 0);
}

export async function getAdditionalRentForNewExtensionLen(
    connection: Connection,
    address: PublicKey,
    e: ExtensionType,
    extensionLen: number,
    programId = TOKEN_2022_PROGRAM_ID,
    accountInfo?: AccountInfo<Buffer> | null
): Promise<number> {
    // If account info was provided, don't fetch again
    const info = accountInfo ?? (await connection.getAccountInfo(address));

    if (!info) {
        throw new TokenAccountNotFoundError();
    }

    const newAccountLen = getNewAccountLenForExtensionLen(info, address, e, extensionLen, programId);

    if (newAccountLen <= info.data.length) {
        return 0;
    }

    const newRentExemptMinimum = await connection.getMinimumBalanceForRentExemption(newAccountLen);

    return newRentExemptMinimum - info.lamports;
}
