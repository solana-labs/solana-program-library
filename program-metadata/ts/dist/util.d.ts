/// <reference types="node" />
import BN from 'bn.js';
export declare class Numberu32 extends BN {
    constructor(n: number);
    /**
   * Convert to Buffer representation
   */
    toBuffer(): Buffer;
    /**
     * Construct a Numberu64 from Buffer representation
     */
    static fromBuffer(buffer: any): BN;
}
export declare class Assignable {
    constructor(properties: any);
}
