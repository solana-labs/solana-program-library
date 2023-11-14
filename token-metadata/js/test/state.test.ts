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

    it('Can pack and unpack as rust implementation with additionalMetadata', () => {
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
        const meta = {
            updateAuthority: new PublicKey('44444444444444444444444444444444444444444444'),
            mint: new PublicKey('55555555555555555555555555555555555555555555'),
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        };

        const bytes = Buffer.from([
            45, 91, 65, 60, 101, 64, 222, 21, 12, 147, 115, 20, 77, 81, 51, 202, 76, 184, 48, 186, 15, 117, 103, 22,
            172, 234, 14, 80, 215, 148, 53, 229, 60, 121, 172, 80, 135, 1, 40, 28, 16, 196, 153, 112, 103, 22, 239, 184,
            102, 74, 235, 162, 191, 71, 52, 30, 59, 226, 189, 193, 31, 112, 71, 220, 4, 0, 0, 0, 110, 97, 109, 101, 6,
            0, 0, 0, 115, 121, 109, 98, 111, 108, 3, 0, 0, 0, 117, 114, 105, 0, 0, 0, 0,
        ]);

        expect(pack(meta)).to.deep.equal(bytes);
        expect(unpack(bytes)).to.deep.equal(meta);
    });

    it('Can pack and unpack with mint, updateAuthority and additional metadata', () => {
        const meta: TokenMetadata = {
            updateAuthority: new PublicKey('44444444444444444444444444444444444444444444'),
            mint: new PublicKey('55555555555555555555555555555555555555555555'),
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [
                ['key1', 'value1'],
                ['key2', 'value2'],
            ],
        };

        const bytes = Buffer.from([
            45, 91, 65, 60, 101, 64, 222, 21, 12, 147, 115, 20, 77, 81, 51, 202, 76, 184, 48, 186, 15, 117, 103, 22,
            172, 234, 14, 80, 215, 148, 53, 229, 60, 121, 172, 80, 135, 1, 40, 28, 16, 196, 153, 112, 103, 22, 239, 184,
            102, 74, 235, 162, 191, 71, 52, 30, 59, 226, 189, 193, 31, 112, 71, 220, 4, 0, 0, 0, 110, 97, 109, 101, 6,
            0, 0, 0, 115, 121, 109, 98, 111, 108, 3, 0, 0, 0, 117, 114, 105, 2, 0, 0, 0, 4, 0, 0, 0, 107, 101, 121, 49,
            6, 0, 0, 0, 118, 97, 108, 117, 101, 49, 4, 0, 0, 0, 107, 101, 121, 50, 6, 0, 0, 0, 118, 97, 108, 117, 101,
            50,
        ]);

        expect(pack(meta)).to.deep.equal(bytes);
        expect(unpack(bytes)).to.deep.equal(meta);
    });
});
