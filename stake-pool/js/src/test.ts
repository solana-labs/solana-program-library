import * as index from './index.js';
import * as schema from './schema.js';
import BN from 'bn.js';
import assert from 'assert';

describe('Array', function () {
    describe('#indexOf()', function () {
        it('should return -1 when the value is not present', () => {
            assert.equal([1, 2, 3].indexOf(4), -1);
        });
    });
});

// describe('schema.decode', () => {
//     describe('StakePoolAccount'. () => {

//     })

//     describe('StakePoolAccount'. () => {

//     })
// })

describe('PrettyPrintPubkey', () => {
    it('should successfully pretty print a pubkey', () => {
        assert.equal(index.prettyPrintPubKey(new schema.PublicKey({ 'value': new BN("99572085579321386496717000324290408927851378839748241098946587626478579848783") })), "6MfzrQUzB2mozveRWU9a77zMoQzSrYa4Gq46KswjupQB");
    });
});
