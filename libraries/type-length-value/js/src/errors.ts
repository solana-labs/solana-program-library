/** Base class for errors */
export abstract class TlvError extends Error {
    constructor(message?: string) {
        super(message);
    }
}

/** Thrown if the byte length of an tlv buffer doesn't match the expected size */
export class TlvInvalidAccountSizeError extends TlvError {
    name = 'TlvInvalidAccountSizeError';
}

/** Thrown if an invalid tlv discriminator is supplied */
export class TlvInvalidDiscriminatorError extends TlvError {
    name = 'TlvInvalidDiscriminatorError';
}
