import { ACCOUNT_SIZE } from '../state/account.js';
import { MINT_SIZE } from '../state/mint.js';
import { MULTISIG_SIZE } from '../state/multisig.js';
import { ACCOUNT_TYPE_SIZE } from './accountType.js';
import { CPI_GUARD_SIZE } from './cpiGuard/index.js';
import { DEFAULT_ACCOUNT_STATE_SIZE } from './defaultAccountState/index.js';
import { IMMUTABLE_OWNER_SIZE } from './immutableOwner.js';
import { INTEREST_BEARING_MINT_CONFIG_STATE_SIZE } from './interestBearingMint/state.js';
import { MEMO_TRANSFER_SIZE } from './memoTransfer/index.js';
import { MINT_CLOSE_AUTHORITY_SIZE } from './mintCloseAuthority.js';
import { NON_TRANSFERABLE_SIZE, NON_TRANSFERABLE_ACCOUNT_SIZE } from './nonTransferable.js';
import { PERMANENT_DELEGATE_SIZE } from './permanentDelegate.js';
import { TRANSFER_FEE_AMOUNT_SIZE, TRANSFER_FEE_CONFIG_SIZE } from './transferFee/index.js';
export var ExtensionType;
(function (ExtensionType) {
    ExtensionType[ExtensionType["Uninitialized"] = 0] = "Uninitialized";
    ExtensionType[ExtensionType["TransferFeeConfig"] = 1] = "TransferFeeConfig";
    ExtensionType[ExtensionType["TransferFeeAmount"] = 2] = "TransferFeeAmount";
    ExtensionType[ExtensionType["MintCloseAuthority"] = 3] = "MintCloseAuthority";
    ExtensionType[ExtensionType["ConfidentialTransferMint"] = 4] = "ConfidentialTransferMint";
    ExtensionType[ExtensionType["ConfidentialTransferAccount"] = 5] = "ConfidentialTransferAccount";
    ExtensionType[ExtensionType["DefaultAccountState"] = 6] = "DefaultAccountState";
    ExtensionType[ExtensionType["ImmutableOwner"] = 7] = "ImmutableOwner";
    ExtensionType[ExtensionType["MemoTransfer"] = 8] = "MemoTransfer";
    ExtensionType[ExtensionType["NonTransferable"] = 9] = "NonTransferable";
    ExtensionType[ExtensionType["InterestBearingConfig"] = 10] = "InterestBearingConfig";
    ExtensionType[ExtensionType["CpiGuard"] = 11] = "CpiGuard";
    ExtensionType[ExtensionType["PermanentDelegate"] = 12] = "PermanentDelegate";
    ExtensionType[ExtensionType["NonTransferableAccount"] = 13] = "NonTransferableAccount";
})(ExtensionType || (ExtensionType = {}));
export const TYPE_SIZE = 2;
export const LENGTH_SIZE = 2;
// NOTE: All of these should eventually use their type's Span instead of these
// constants.  This is provided for at least creation to work.
export function getTypeLen(e) {
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
        case ExtensionType.NonTransferable:
            return NON_TRANSFERABLE_SIZE;
        case ExtensionType.InterestBearingConfig:
            return INTEREST_BEARING_MINT_CONFIG_STATE_SIZE;
        case ExtensionType.PermanentDelegate:
            return PERMANENT_DELEGATE_SIZE;
        case ExtensionType.NonTransferableAccount:
            return NON_TRANSFERABLE_ACCOUNT_SIZE;
        default:
            throw Error(`Unknown extension type: ${e}`);
    }
}
export function isMintExtension(e) {
    switch (e) {
        case ExtensionType.TransferFeeConfig:
        case ExtensionType.MintCloseAuthority:
        case ExtensionType.ConfidentialTransferMint:
        case ExtensionType.DefaultAccountState:
        case ExtensionType.NonTransferable:
        case ExtensionType.InterestBearingConfig:
        case ExtensionType.PermanentDelegate:
            return true;
        case ExtensionType.Uninitialized:
        case ExtensionType.TransferFeeAmount:
        case ExtensionType.ConfidentialTransferAccount:
        case ExtensionType.ImmutableOwner:
        case ExtensionType.MemoTransfer:
        case ExtensionType.CpiGuard:
        case ExtensionType.NonTransferableAccount:
            return false;
        default:
            throw Error(`Unknown extension type: ${e}`);
    }
}
export function isAccountExtension(e) {
    switch (e) {
        case ExtensionType.TransferFeeAmount:
        case ExtensionType.ConfidentialTransferAccount:
        case ExtensionType.ImmutableOwner:
        case ExtensionType.MemoTransfer:
        case ExtensionType.CpiGuard:
        case ExtensionType.NonTransferableAccount:
            return true;
        case ExtensionType.Uninitialized:
        case ExtensionType.TransferFeeConfig:
        case ExtensionType.MintCloseAuthority:
        case ExtensionType.ConfidentialTransferMint:
        case ExtensionType.DefaultAccountState:
        case ExtensionType.NonTransferable:
        case ExtensionType.InterestBearingConfig:
        case ExtensionType.PermanentDelegate:
            return false;
        default:
            throw Error(`Unknown extension type: ${e}`);
    }
}
export function getAccountTypeOfMintType(e) {
    switch (e) {
        case ExtensionType.TransferFeeConfig:
            return ExtensionType.TransferFeeAmount;
        case ExtensionType.ConfidentialTransferMint:
            return ExtensionType.ConfidentialTransferAccount;
        case ExtensionType.NonTransferable:
            return ExtensionType.NonTransferableAccount;
        case ExtensionType.TransferFeeAmount:
        case ExtensionType.ConfidentialTransferAccount:
        case ExtensionType.CpiGuard:
        case ExtensionType.DefaultAccountState:
        case ExtensionType.ImmutableOwner:
        case ExtensionType.MemoTransfer:
        case ExtensionType.MintCloseAuthority:
        case ExtensionType.Uninitialized:
        case ExtensionType.InterestBearingConfig:
        case ExtensionType.PermanentDelegate:
        case ExtensionType.NonTransferableAccount:
            return ExtensionType.Uninitialized;
    }
}
function getLen(extensionTypes, baseSize) {
    if (extensionTypes.length === 0) {
        return baseSize;
    }
    else {
        const accountLength = ACCOUNT_SIZE +
            ACCOUNT_TYPE_SIZE +
            extensionTypes
                .filter((element, i) => i === extensionTypes.indexOf(element))
                .map((element) => getTypeLen(element) + TYPE_SIZE + LENGTH_SIZE)
                .reduce((a, b) => a + b);
        if (accountLength === MULTISIG_SIZE) {
            return accountLength + TYPE_SIZE;
        }
        else {
            return accountLength;
        }
    }
}
export function getMintLen(extensionTypes) {
    return getLen(extensionTypes, MINT_SIZE);
}
export function getAccountLen(extensionTypes) {
    return getLen(extensionTypes, ACCOUNT_SIZE);
}
export function getExtensionData(extension, tlvData) {
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
export function getExtensionTypes(tlvData) {
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
export function getAccountLenForMint(mint) {
    const extensionTypes = getExtensionTypes(mint.tlvData);
    const accountExtensions = extensionTypes.map(getAccountTypeOfMintType);
    return getAccountLen(accountExtensions);
}
//# sourceMappingURL=extensionType.js.map