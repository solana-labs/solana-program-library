import { expect } from 'chai';
import { SplDiscriminator } from '../../src/extensions/splDiscriminate';
import { createHash } from 'crypto';

describe('splDiscrimintor', () => {
    const testVectors = [
        'hello',
        'this-is-a-test',
        'test-namespace:this-is-a-test',
        'test-namespace:this-is-a-test:with-a-longer-name',
    ];

    const testExpectedBytes = testVectors.map((x) => {
        return createHash('sha256').update(x).digest();
    });

    const testSplDiscriminator = (length: number) => {
        for (let i = 0; i < testVectors.length; i++) {
            const discriminator = new SplDiscriminator(testVectors[i], length);
            const expectedBytes = testExpectedBytes[i].subarray(0, length);
            expect(discriminator.bytes).to.have.length(length);
            expect(discriminator.bytes).to.deep.equal(expectedBytes);
        }
    };

    it('should produce the expected bytes', () => {
        testSplDiscriminator(8);
        testSplDiscriminator(4);
        testSplDiscriminator(2);
    });
});
