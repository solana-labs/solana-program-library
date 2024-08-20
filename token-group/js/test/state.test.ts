import { PublicKey } from '@solana/web3.js';
import { expect } from 'chai';

import type { TokenGroup, TokenGroupMember } from '../src/state';
import { unpackTokenGroupMember, packTokenGroupMember, unpackTokenGroup, packTokenGroup } from '../src';

describe('Token Group State', () => {
    describe('Token Group', () => {
        function checkPackUnpack(tokenGroup: TokenGroup) {
            const packed = packTokenGroup(tokenGroup);
            const unpacked = unpackTokenGroup(packed);
            expect(unpacked).to.deep.equal(tokenGroup);
        }

        it('Can pack and unpack TokenGroup with updateAuthoritygroup', () => {
            checkPackUnpack({
                mint: new PublicKey('44444444444444444444444444444444444444444444'),
                updateAuthority: new PublicKey('55555555555555555555555555555555555555555555'),
                size: BigInt(10),
                maxSize: BigInt(20),
            });
        });

        it('Can pack and unpack TokenGroup without updateAuthoritygroup', () => {
            checkPackUnpack({
                mint: new PublicKey('44444444444444444444444444444444444444444444'),
                size: BigInt(10),
                maxSize: BigInt(20),
            });
        });
    });

    describe('Token Group Member', () => {
        function checkPackUnpack(tokenGroupMember: TokenGroupMember) {
            const packed = packTokenGroupMember(tokenGroupMember);
            const unpacked = unpackTokenGroupMember(packed);
            expect(unpacked).to.deep.equal(tokenGroupMember);
        }
        it('Can pack and unpack TokenGroupMembergroup', () => {
            checkPackUnpack({
                mint: new PublicKey('55555555555555555555555555555555555555555555'),
                group: new PublicKey('66666666666666666666666666666666666666666666'),
                memberNumber: BigInt(8),
            });
        });
    });
});
