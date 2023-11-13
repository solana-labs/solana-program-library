import { PublicKey } from '@solana/web3.js';
import { expect } from 'chai';

import type { TokenMetadata } from '../src/state';
import { unpack, pack } from '../src';

describe('Token Metadata State', () => {
    it('Can pack and unpack as rust implementation', () => {
        const meta = {
            mint: PublicKey.default,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        };

        // From rust implementation
        const bytes = Buffer.from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 110, 97,
            109, 101, 6, 0, 0, 0, 115, 121, 109, 98, 111, 108, 3, 0, 0, 0, 117, 114, 105, 0, 0, 0, 0,
        ]);

        expect(pack(meta)).to.deep.equal(bytes);
        expect(unpack(bytes)).to.deep.equal(meta);
    });

    it('Can unpack with additionalMetadata', () => {
        const meta: TokenMetadata = {
            mint: PublicKey.default,
            name: 'new_name',
            symbol: 'new_symbol',
            uri: 'new_uri',
            additionalMetadata: [
                ['key1', 'value1'],
                ['key2', 'value2'],
            ],
        };
        // From rust implementation
        const bytes = Buffer.from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 110, 101,
            119, 95, 110, 97, 109, 101, 10, 0, 0, 0, 110, 101, 119, 95, 115, 121, 109, 98, 111, 108, 7, 0, 0, 0, 110,
            101, 119, 95, 117, 114, 105, 2, 0, 0, 0, 4, 0, 0, 0, 107, 101, 121, 49, 6, 0, 0, 0, 118, 97, 108, 117, 101,
            49, 4, 0, 0, 0, 107, 101, 121, 50, 6, 0, 0, 0, 118, 97, 108, 117, 101, 50,
        ]);

        expect(pack(meta)).to.deep.equal(bytes);
        expect(unpack(bytes)).to.deep.equal(meta);
    });

    it('Can pack and unpack with mint and updateAuthority', () => {
        const input = pack({
            updateAuthority: new PublicKey('44444444444444444444444444444444444444444444'),
            mint: new PublicKey('55555555555555555555555555555555555555555555'),
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });

        const meta = unpack(input);
        expect(meta).to.deep.equal({
            updateAuthority: new PublicKey('44444444444444444444444444444444444444444444'),
            mint: new PublicKey('55555555555555555555555555555555555555555555'),
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });
});
