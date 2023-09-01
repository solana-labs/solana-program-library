/** Base class for errors */
export declare abstract class TokenError extends Error {
    constructor(message?: string);
}
/** Thrown if an account is not found at the expected address */
export declare class TokenAccountNotFoundError extends TokenError {
    name: string;
}
/** Thrown if a program state account is not a valid Account */
export declare class TokenInvalidAccountError extends TokenError {
    name: string;
}
/** Thrown if a program state account is not owned by the expected token program */
export declare class TokenInvalidAccountOwnerError extends TokenError {
    name: string;
}
/** Thrown if the byte length of an program state account doesn't match the expected size */
export declare class TokenInvalidAccountSizeError extends TokenError {
    name: string;
}
/** Thrown if the mint of a token account doesn't match the expected mint */
export declare class TokenInvalidMintError extends TokenError {
    name: string;
}
/** Thrown if the owner of a token account doesn't match the expected owner */
export declare class TokenInvalidOwnerError extends TokenError {
    name: string;
}
/** Thrown if the owner of a token account is a PDA (Program Derived Address) */
export declare class TokenOwnerOffCurveError extends TokenError {
    name: string;
}
/** Thrown if an instruction's program is invalid */
export declare class TokenInvalidInstructionProgramError extends TokenError {
    name: string;
}
/** Thrown if an instruction's keys are invalid */
export declare class TokenInvalidInstructionKeysError extends TokenError {
    name: string;
}
/** Thrown if an instruction's data is invalid */
export declare class TokenInvalidInstructionDataError extends TokenError {
    name: string;
}
/** Thrown if an instruction's type is invalid */
export declare class TokenInvalidInstructionTypeError extends TokenError {
    name: string;
}
/** Thrown if the program does not support the desired instruction */
export declare class TokenUnsupportedInstructionError extends TokenError {
    name: string;
}
//# sourceMappingURL=errors.d.ts.map