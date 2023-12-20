import { PublicKey } from '@solana/web3.js';
import { getBytesCodec, getStructCodec } from '@solana/codecs-data-structures';
import { PodU32 } from './tokenGroup.js';

const tokenGroupMemberCodec = getStructCodec([
    ['mint', getBytesCodec({ size: 32 })],
    ['group', getBytesCodec({ size: 32 })],
    ['memberNumber', getBytesCodec({ size: 4 })],
]);

export interface TokenGroupMember {
    /** The associated mint, used to counter spoofing to be sure that member belongs to a particular mint */
    mint: PublicKey;
    /** The pubkey of the `TokenGroup` */
    group: PublicKey;
    /** The member number */
    memberNumber: PodU32;
}

// Pack TokenGroupMember into byte slab
export const packTokenGroupMember = (member: TokenGroupMember): Uint8Array => {
    return tokenGroupMemberCodec.encode({
        mint: member.mint.toBuffer(),
        group: member.group.toBuffer(),
        memberNumber: member.memberNumber.toBuffer(),
    });
};

// unpack byte slab into TokenGroupMember
export function unpackTokenGroupMember(buffer: Buffer | Uint8Array): TokenGroupMember {
    const data = tokenGroupMemberCodec.decode(buffer);
    return {
        mint: new PublicKey(data.mint),
        group: new PublicKey(data.group),
        memberNumber: new PodU32(data.memberNumber),
    };
}
