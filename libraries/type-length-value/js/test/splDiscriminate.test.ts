import { expect } from 'chai';
import { splDiscriminate } from '../src/splDiscriminate';

const testVectors = [
    'hello',
    'this-is-a-test',
    'test-namespace:this-is-a-test',
    'test-namespace:this-is-a-test:with-a-longer-name',
];

const testExpectedBytes = await Promise.all(
    testVectors.map(x =>
        crypto.subtle.digest('SHA-256', new TextEncoder().encode(x)).then(digest => new Uint8Array(digest)),
    ),
);

describe('splDiscrimintor', () => {
    const testSplDiscriminator = async (length: number) => {
        for (let i = 0; i < testVectors.length; i++) {
            const discriminator = await splDiscriminate(testVectors[i], length);
            const expectedBytes = testExpectedBytes[i].subarray(0, length);
            expect(discriminator).to.have.length(length);
            expect(discriminator).to.deep.equal(expectedBytes);
        }
    };

    it('should produce the expected bytes', () => {
        testSplDiscriminator(8);
        testSplDiscriminator(4);
        testSplDiscriminator(2);
    });

    it('should produce the same bytes as rust library', async () => {
        const expectedBytes = Buffer.from([105, 37, 101, 197, 75, 251, 102, 26]);
        const discriminator = await splDiscriminate('spl-transfer-hook-interface:execute');
        expect(discriminator).to.deep.equal(expectedBytes);
    });
});
