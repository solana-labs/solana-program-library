import { ACCOUNT_SIZE } from './account';
import { MULTISIG_SIZE } from './multisig';

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
}

export const ACCOUNT_TYPE_SIZE = 1;
export const TYPE_SIZE = 2;
export const LENGTH_SIZE = 2;

// NOTE: All of these should eventually use their type's Span instead of these
// constants.  This is provided for at least creation to work.
export function getTypeLen(e: ExtensionType): number {
    switch (e) {
        case ExtensionType.Uninitialized:
            return 0;
        case ExtensionType.TransferFeeConfig:
            return 108;
        case ExtensionType.TransferFeeAmount:
            return 8;
        case ExtensionType.MintCloseAuthority:
            return 32;
        case ExtensionType.ConfidentialTransferMint:
            return 97;
        case ExtensionType.ConfidentialTransferAccount:
            return 286;
        case ExtensionType.DefaultAccountState:
            return 1;
        case ExtensionType.ImmutableOwner:
            return 0;
        case ExtensionType.MemoTransfer:
            return 1;
        default:
            throw Error(`Unknown extension type: ${e}`);
    }
}

export function getAccountLen(extensionTypes: ExtensionType[]): number {
    if (extensionTypes.length === 0) {
        return ACCOUNT_SIZE;
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
