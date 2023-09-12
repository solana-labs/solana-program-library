/** Base class for errors */
export abstract class TlvError extends Error {
    constructor(message?: string) {
        super(message);
    }
}

/** Thrown if the byte length of an tlv buffer doesn't match the expected size */
export class TlvInvalidAccountDataError extends TlvError {
    name = 'TlvInvalidAccountDataError';
}
