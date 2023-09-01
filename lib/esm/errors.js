/** Base class for errors */
export class TokenError extends Error {
    constructor(message) {
        super(message);
    }
}
/** Thrown if an account is not found at the expected address */
export class TokenAccountNotFoundError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenAccountNotFoundError';
    }
}
/** Thrown if a program state account is not a valid Account */
export class TokenInvalidAccountError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenInvalidAccountError';
    }
}
/** Thrown if a program state account is not owned by the expected token program */
export class TokenInvalidAccountOwnerError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenInvalidAccountOwnerError';
    }
}
/** Thrown if the byte length of an program state account doesn't match the expected size */
export class TokenInvalidAccountSizeError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenInvalidAccountSizeError';
    }
}
/** Thrown if the mint of a token account doesn't match the expected mint */
export class TokenInvalidMintError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenInvalidMintError';
    }
}
/** Thrown if the owner of a token account doesn't match the expected owner */
export class TokenInvalidOwnerError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenInvalidOwnerError';
    }
}
/** Thrown if the owner of a token account is a PDA (Program Derived Address) */
export class TokenOwnerOffCurveError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenOwnerOffCurveError';
    }
}
/** Thrown if an instruction's program is invalid */
export class TokenInvalidInstructionProgramError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenInvalidInstructionProgramError';
    }
}
/** Thrown if an instruction's keys are invalid */
export class TokenInvalidInstructionKeysError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenInvalidInstructionKeysError';
    }
}
/** Thrown if an instruction's data is invalid */
export class TokenInvalidInstructionDataError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenInvalidInstructionDataError';
    }
}
/** Thrown if an instruction's type is invalid */
export class TokenInvalidInstructionTypeError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenInvalidInstructionTypeError';
    }
}
/** Thrown if the program does not support the desired instruction */
export class TokenUnsupportedInstructionError extends TokenError {
    constructor() {
        super(...arguments);
        this.name = 'TokenUnsupportedInstructionError';
    }
}
//# sourceMappingURL=errors.js.map