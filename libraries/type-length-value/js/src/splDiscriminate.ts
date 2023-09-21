import { createHash } from 'crypto';

export const splDiscriminate = (discriminator: string, length = 8): Buffer => {
    const digest = createHash('sha256').update(discriminator).digest();
    return digest.subarray(0, length);
};
