import { PublicKey } from '@solana/web3.js';
import { expect } from 'chai';

import type { TokenMetadata } from '@solana/spl-token-metadata';
import { Field } from '@solana/spl-token-metadata';
import { getNormalizedTokenMetadataField, updateTokenMetadata } from '../../src';

describe('SPL Token 2022 Metadata Extension', () => {
    it('can get normalized token metadata field', async () => {
        expect(getNormalizedTokenMetadataField('name')).to.equal('name');
        expect(getNormalizedTokenMetadataField('Name')).to.equal('name');
        expect(getNormalizedTokenMetadataField(Field.Name)).to.equal('name');

        expect(getNormalizedTokenMetadataField('symbol')).to.equal('symbol');
        expect(getNormalizedTokenMetadataField('Symbol')).to.equal('symbol');
        expect(getNormalizedTokenMetadataField(Field.Symbol)).to.equal('symbol');

        expect(getNormalizedTokenMetadataField('uri')).to.equal('uri');
        expect(getNormalizedTokenMetadataField('Uri')).to.equal('uri');
        expect(getNormalizedTokenMetadataField(Field.Uri)).to.equal('uri');

        expect(getNormalizedTokenMetadataField('mint')).to.equal('mint');
        expect(getNormalizedTokenMetadataField('updateAuthority')).to.equal('updateAuthority');
        expect(getNormalizedTokenMetadataField('Key1')).to.equal('Key1');
        expect(getNormalizedTokenMetadataField('KEY_1')).to.equal('KEY_1');
    });

    describe('Update token metadata', () => {
        it('can guard against invalid values', () => {
            const input = Object.freeze({
                mint: PublicKey.default,
                updateAuthority: PublicKey.unique(),
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [],
            } as TokenMetadata);

            expect(() => updateTokenMetadata(input, 'mint', null)).to.throw(
                'TokenMetadata field mint must be a PublicKey'
            );
            expect(() => updateTokenMetadata(input, 'mint', 'asd')).to.throw(
                'TokenMetadata field mint must be a PublicKey'
            );

            expect(() => updateTokenMetadata(input, 'updateAuthority', 'string')).to.throw(
                'TokenMetadata field updateAuthority must be a PublicKey or null'
            );

            expect(() => updateTokenMetadata(input, 'name', null)).to.throw('TokenMetadata value must be a string');
            expect(() => updateTokenMetadata(input, 'name', PublicKey.unique())).to.throw(
                'TokenMetadata value must be a string'
            );

            expect(() => updateTokenMetadata(input, 'key1', null)).to.throw('TokenMetadata value must be a string');
            expect(() => updateTokenMetadata(input, 'key1', PublicKey.unique())).to.throw(
                'TokenMetadata value must be a string'
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

        it('can update mint', async () => {
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

            const newMint = PublicKey.unique();

            const expected: TokenMetadata = {
                mint: newMint,
                name: 'new_name',
                symbol: 'new_symbol',
                uri: 'new_uri',
                additionalMetadata: [
                    ['key1', 'value1'],
                    ['key2', 'value2'],
                ],
            };

            expect(updateTokenMetadata(input, 'mint', newMint)).to.deep.equal(expected);
        });

        it('can remove updateAuthority', async () => {
            const input = Object.freeze({
                mint: PublicKey.default,
                name: 'new_name',
                symbol: 'new_symbol',
                updateAuthority: PublicKey.unique(),
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
                ],
            };

            expect(updateTokenMetadata(input, 'updateAuthority', null)).to.deep.equal(expected);
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
