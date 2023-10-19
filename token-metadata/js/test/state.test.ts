import { PublicKey } from '@solana/web3.js';
import { serialize } from 'borsh';
import { expect } from 'chai';

import { TokenMetadata, TokenMetadataDiscriminate, schema, unpack } from '../src/state';

describe('Token Metadata State', () => {
    const lengthBuffer = (buffer: Buffer | Uint8Array): Buffer => {
        const length = Buffer.alloc(4);
        length.writeUIntLE(buffer.length, 0, 4);
        return length;
    };

    // Helper function to pack meta into tlv bytes slab
    const pack = (meta: TokenMetadata) => {
        const data = serialize(schema, {
            ...meta,
            updateAuthority: meta.updateAuthority?.toBuffer(),
            mint: meta.mint.toBuffer(),
        });
        return Buffer.concat([TokenMetadataDiscriminate, lengthBuffer(data), data]);
    };

    it('Can unpack', () => {
        const data = Buffer.from([
            // From rust implementation
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 110, 97,
            109, 101, 6, 0, 0, 0, 115, 121, 109, 98, 111, 108, 3, 0, 0, 0, 117, 114, 105, 0, 0, 0, 0,
        ]);

        const input = Buffer.concat([TokenMetadataDiscriminate, lengthBuffer(data), data]);

        const meta = unpack(input);
        expect(meta).to.deep.equal({
            mint: PublicKey.default,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('Can unpack with additionalMetadata', () => {
        const data = Buffer.from([
            // From rust implementation
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 110, 101,
            119, 95, 110, 97, 109, 101, 10, 0, 0, 0, 110, 101, 119, 95, 115, 121, 109, 98, 111, 108, 7, 0, 0, 0, 110,
            101, 119, 95, 117, 114, 105, 2, 0, 0, 0, 4, 0, 0, 0, 107, 101, 121, 49, 6, 0, 0, 0, 118, 97, 108, 117, 101,
            49, 4, 0, 0, 0, 107, 101, 121, 50, 6, 0, 0, 0, 118, 97, 108, 117, 101, 50,
        ]);

        const input = Buffer.concat([TokenMetadataDiscriminate, lengthBuffer(data), data]);
        const meta = unpack(input);
        expect(meta).to.deep.equal({
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
