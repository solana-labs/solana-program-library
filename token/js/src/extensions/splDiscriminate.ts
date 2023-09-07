import type { TlvDiscriminator } from './tlvState.js';
import { createHash } from 'crypto';

export class SplDiscriminator implements TlvDiscriminator {
    private readonly discriminator: string;
    private readonly length: number;

    public constructor(discriminator: string, length = 8) {
        this.discriminator = discriminator;
        this.length = length;
    }

    public get bytes(): Buffer {
        const digest = createHash('sha256').update(this.discriminator).digest();
        return digest.subarray(0, this.length);
    }
}
