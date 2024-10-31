import { assertDigestCapabilityIsAvailable } from '@solana/assertions';

export async function splDiscriminate(discriminator: string, length = 8): Promise<Uint8Array> {
    assertDigestCapabilityIsAvailable();
    const bytes = new TextEncoder().encode(discriminator);
    const digest = await crypto.subtle.digest('SHA-256', bytes);
    return new Uint8Array(digest).subarray(0, length);
}
