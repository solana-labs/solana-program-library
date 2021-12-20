/** Base class for errors */
export abstract class TokenError extends Error {
    constructor(message?: string) {
        super(message);
    }
}

/** Thrown if an account is not found at the expected address */
export class TokenAccountNotFoundError extends TokenError {
    name = 'TokenAccountNotFoundError';
}

/** Thrown if a program state account is not owned by the expected token program */
export class TokenInvalidAccountOwnerError extends TokenError {
    name = 'TokenInvalidAccountOwnerError';
}

/** Thrown if the byte length of an program state account doesn't match the expected size */
export class TokenInvalidAccountSizeError extends TokenError {
    name = 'TokenInvalidAccountSizeError';
}

/** Thrown if the mint of a token account doesn't match the expected mint */
export class TokenInvalidMintError extends TokenError {
    name = 'TokenInvalidMintError';
}

/** Thrown if the owner of a token account doesn't match the expected owner */
export class TokenInvalidOwnerError extends TokenError {
    name = 'TokenInvalidOwnerError';
}

/** Thrown if the owner of a token account is a PDA (Program Derived Address) */
export class TokenOwnerOffCurveError extends TokenError {
    name = 'TokenOwnerOffCurveError';
}
