import { PublicKey } from '@solana/web3.js';
import { expect } from 'chai';

import type { TokenMetadata } from '@solana/spl-token-metadata';
import { Field } from '@solana/spl-token-metadata';
import { updateTokenMetadata } from '../../src';

describe('SPL Token 2022 Metadata Extension', () => {
    describe('Update token metadata', () => {
        it('guards against updates on mint or updateAuthority', async () => {
            const input = Object.freeze({
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            } as TokenMetadata);

            expect(() => updateTokenMetadata(input, 'mint', 'string')).to.throw(
                'Cannot update mint via this instruction'
            );
            expect(() => updateTokenMetadata(input, 'updateAuthority', 'string')).to.throw(
                'Cannot update updateAuthority via this instruction'
            );
        });
        it('can update name', async () => {
            const input = Object.freeze({
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            } as TokenMetadata);

            const expected: TokenMetadata = {
                mint: PublicKey.default,
                name: 'updated_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            };

            expect(updateTokenMetadata(input, 'name', 'updated_name')).to.deep.equal(expected);
            expect(updateTokenMetadata(input, 'Name', 'updated_name')).to.deep.equal(expected);
            expect(updateTokenMetadata(input, Field.Name, 'updated_name')).to.deep.equal(expected);
        });

        it('can update symbol', async () => {
            const input = Object.freeze({
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            } as TokenMetadata);

            const expected: TokenMetadata = {
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'updated_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            };

            expect(updateTokenMetadata(input, 'symbol', 'updated_symbol')).to.deep.equal(expected);
            expect(updateTokenMetadata(input, 'Symbol', 'updated_symbol')).to.deep.equal(expected);
            expect(updateTokenMetadata(input, Field.Symbol, 'updated_symbol')).to.deep.equal(expected);
        });

        it('can update uri', async () => {
            const input = Object.freeze({
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            } as TokenMetadata);

            const expected: TokenMetadata = {
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'updated_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            };

            expect(updateTokenMetadata(input, 'uri', 'updated_uri')).to.deep.equal(expected);
            expect(updateTokenMetadata(input, 'Uri', 'updated_uri')).to.deep.equal(expected);
            expect(updateTokenMetadata(input, Field.Uri, 'updated_uri')).to.deep.equal(expected);
        });

        it('can update additional Metadata', async () => {
            const input = Object.freeze({
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            } as TokenMetadata);

            const expected: TokenMetadata = {
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'update1'],
                    ['key2', 'value2'],
                ],
            };

            expect(updateTokenMetadata(input, 'key1', 'update1')).to.deep.equal(expected);
        });

        it('can add additional Metadata', async () => {
            const input = Object.freeze({
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            } as TokenMetadata);

            const expected: TokenMetadata = {
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                    ['key3', 'value3'],
                ],
            };

            expect(updateTokenMetadata(input, 'key3', 'value3')).to.deep.equal(expected);
        });

        it('can update `additionalMetadata` key to additional metadata', async () => {
            const input = Object.freeze({
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                    ['additionalMetadata', 'value3'],
                ],
            } as TokenMetadata);

            const expected: TokenMetadata = {
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                    ['additionalMetadata', 'update3'],
                ],
            };

            expect(updateTokenMetadata(input, 'additionalMetadata', 'update3')).to.deep.equal(expected);
        });

        it('can add `additionalMetadata` key to additional metadata', async () => {
            const input = Object.freeze({
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            } as TokenMetadata);

            const expected: TokenMetadata = {
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                    ['additionalMetadata', 'value3'],
                ],
            };

            expect(updateTokenMetadata(input, 'additionalMetadata', 'value3')).to.deep.equal(expected);
        });
    });
});
