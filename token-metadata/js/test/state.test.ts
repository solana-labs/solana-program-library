import { PublicKey } from '@solana/web3.js';
import { expect } from 'chai';

import type { TokenMetadata } from '../src/state';
import { unpack, pack } from '../src';

function checkPackUnpack(tokenMetadata: TokenMetadata) {
    const packed = pack(tokenMetadata);
    const unpacked = unpack(packed);
    expect(unpacked).to.deep.equal(tokenMetadata);
}

describe('Token Metadata State', () => {
    it('Can pack and unpack base token metadata', () => {
        checkPackUnpack({
            mint: PublicKey.default,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('Can pack and unpack with updateAuthority', () => {
        checkPackUnpack({
            updateAuthority: new PublicKey('44444444444444444444444444444444444444444444'),
            mint: new PublicKey('55555555555555555555555555555555555555555555'),
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('Can pack and unpack with additional metadata', () => {
        checkPackUnpack({
            mint: PublicKey.default,
            name: 'new_name',
            symbol: 'new_symbol',
            uri: 'new_uri',
            additionalMetadata: [
                ['key1', 'value1'],
                ['key2', 'value2'],
            ],
        });
    });

    it('Can pack and unpack with updateAuthority and additional metadata', () => {
        checkPackUnpack({
            updateAuthority: new PublicKey('44444444444444444444444444444444444444444444'),
            mint: new PublicKey('55555555555555555555555555555555555555555555'),
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [
                ['key1', 'value1'],
                ['key2', 'value2'],
            ],
        });
    });
});
