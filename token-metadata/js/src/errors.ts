/** Base class for errors */
export abstract class TokenMetadataError extends Error {
    constructor(message?: string) {
        super(message);
    }
}

/** Thrown if incorrect account provided */
export class IncorrectAccountError extends TokenMetadataError {
    name = 'IncorrectAccountError';
}

/** Thrown if Mint has no mint authority */
export class MintHasNoMintAuthorityError extends TokenMetadataError {
    name = 'MintHasNoMintAuthorityError';
}

/** Thrown if Incorrect mint authority has signed the instruction */
export class IncorrectMintAuthorityError extends TokenMetadataError {
    name = 'IncorrectMintAuthorityError';
}

/** Thrown if Token metadata has no update authority */
export class ImmutableMetadataError extends TokenMetadataError {
    name = 'ImmutableMetadataError';
}

/** Thrown if Key not found in metadata account */
export class KeyNotFoundError extends TokenMetadataError {
    name = 'KeyNotFoundError';
}
