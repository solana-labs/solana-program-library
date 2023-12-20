import { PublicKey } from '@solana/web3.js';
import { expect } from 'chai';

import type { TokenGroup, TokenGroupMember } from '../src/state';
import { unpackTokenGroupMember, packTokenGroupMember, unpackTokenGroup, packTokenGroup, PodU32 } from '../src';

describe('State', () => {
    describe('Token Group', () => {
        it('Can pack and unpack TokenGroup with updateAuthority as rust implementation', () => {
            const tokenGroup: TokenGroup = {
                mint: new PublicKey('44444444444444444444444444444444444444444444'),
                updateAuthority: new PublicKey('55555555555555555555555555555555555555555555'),
                size: new PodU32(10),
                maxSize: new PodU32(20),
            };

            // From rust implementation
            const bytes = Uint8Array.from([
                60, 121, 172, 80, 135, 1, 40, 28, 16, 196, 153, 112, 103, 22, 239, 184, 102, 74, 235, 162, 191, 71, 52,
                30, 59, 226, 189, 193, 31, 112, 71, 220, 45, 91, 65, 60, 101, 64, 222, 21, 12, 147, 115, 20, 77, 81, 51,
                202, 76, 184, 48, 186, 15, 117, 103, 22, 172, 234, 14, 80, 215, 148, 53, 229, 10, 0, 0, 0, 20, 0, 0, 0,
            ]);

            expect(packTokenGroup(tokenGroup)).to.deep.equal(bytes);
            expect(unpackTokenGroup(bytes)).to.deep.equal(tokenGroup);
        });

        it('Can pack and unpack TokenGroup without updateAuthority as rust implementation', () => {
            const tokenGroup: TokenGroup = {
                mint: new PublicKey('44444444444444444444444444444444444444444444'),
                size: new PodU32(10),
                maxSize: new PodU32(20),
            };

            // From rust implementation
            const bytes = Uint8Array.from([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 45, 91,
                65, 60, 101, 64, 222, 21, 12, 147, 115, 20, 77, 81, 51, 202, 76, 184, 48, 186, 15, 117, 103, 22, 172,
                234, 14, 80, 215, 148, 53, 229, 10, 0, 0, 0, 20, 0, 0, 0,
            ]);

            expect(packTokenGroup(tokenGroup)).to.deep.equal(bytes);
            expect(unpackTokenGroup(bytes)).to.deep.equal(tokenGroup);
        });
    });

    describe('Token Group Member', () => {
        it('Can pack and unpack TokenGroupMember as rust implementation', () => {
            const tokenGroupMember: TokenGroupMember = {
                mint: new PublicKey('55555555555555555555555555555555555555555555'),
                group: new PublicKey('66666666666666666666666666666666666666666666'),
                memberNumber: new PodU32(8),
            };
            // From rust implementation
            const bytes = Uint8Array.from([
                60, 121, 172, 80, 135, 1, 40, 28, 16, 196, 153, 112, 103, 22, 239, 184, 102, 74, 235, 162, 191, 71, 52,
                30, 59, 226, 189, 193, 31, 112, 71, 220, 75, 152, 23, 100, 168, 193, 114, 35, 20, 245, 191, 204, 128,
                220, 171, 166, 127, 221, 166, 139, 111, 25, 1, 37, 202, 219, 109, 49, 103, 76, 89, 211, 8, 0, 0, 0,
            ]);

            expect(packTokenGroupMember(tokenGroupMember)).to.deep.equal(bytes);
            expect(unpackTokenGroupMember(bytes)).to.deep.equal(tokenGroupMember);
        });
    });
});
