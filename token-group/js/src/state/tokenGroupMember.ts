import { PublicKey } from '@solana/web3.js';
import { getBytesCodec, getStructCodec, getU32Codec } from '@solana/codecs';

const tokenGroupMemberCodec = getStructCodec([
    ['mint', getBytesCodec({ size: 32 })],
    ['group', getBytesCodec({ size: 32 })],
    ['memberNumber', getU32Codec()],
]);

export const TOKEN_GROUP_MEMBER_SIZE = tokenGroupMemberCodec.fixedSize;

export interface TokenGroupMember {
    /** The associated mint, used to counter spoofing to be sure that member belongs to a particular mint */
    mint: PublicKey;
    /** The pubkey of the `TokenGroup` */
    group: PublicKey;
    /** The member number */
    memberNumber: number;
}

// Pack TokenGroupMember into byte slab
export function packTokenGroupMember(member: TokenGroupMember): Uint8Array {
    return tokenGroupMemberCodec.encode({
        mint: member.mint.toBuffer(),
        group: member.group.toBuffer(),
        memberNumber: member.memberNumber,
    });
}

// unpack byte slab into TokenGroupMember
export function unpackTokenGroupMember(buffer: Buffer | Uint8Array): TokenGroupMember {
    const data = tokenGroupMemberCodec.decode(buffer);
    return {
        mint: new PublicKey(data.mint),
        group: new PublicKey(data.group),
        memberNumber: data.memberNumber,
    };
}
