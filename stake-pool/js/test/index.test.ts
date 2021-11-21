import BN from 'bn.js';
import {assert} from 'chai';
import {PublicKey} from '@solana/web3.js';
import * as index from '../src/index';

describe('index', () => {
  it('should successfully pretty print a pubkey', () => {
    assert.equal(
      index.prettyPrintPubKey(
        new PublicKey(
          new BN(
            '99572085579321386496717000324290408927851378839748241098946587626478579848783',
          ),
        ),
      ),
      '6MfzrQUzB2mozveRWU9a77zMoQzSrYa4Gq46KswjupQB',
    );
  });
});
