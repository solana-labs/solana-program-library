import BN from 'bn.js';
import { PublicKey } from '@solana/web3.js';
import * as index from '../src';

describe('index', () => {
  it('should successfully pretty print a pubkey', () => {
    expect(
      index.prettyPrintPubKey(
        new PublicKey(
          new BN(
            '99572085579321386496717000324290408927851378839748241098946587626478579848783',
          ),
        ),
      ),
    ).toEqual('6MfzrQUzB2mozveRWU9a77zMoQzSrYa4Gq46KswjupQB');
  });
});
