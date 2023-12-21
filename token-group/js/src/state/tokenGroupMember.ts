import { PublicKey } from '@solana/web3.js';
import { getBytesCodec, getStructCodec } from '@solana/codecs-data-structures';
import { numberToU32Buffer } from './tokenGroup.js';

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
    memberNumber: number;
}

// Pack TokenGroupMember into byte slab
export const packTokenGroupMember = (member: TokenGroupMember): Uint8Array => {
    return tokenGroupMemberCodec.encode({
        mint: member.mint.toBuffer(),
        group: member.group.toBuffer(),
        memberNumber: numberToU32Buffer(member.memberNumber),
    });
};

// unpack byte slab into TokenGroupMember
export function unpackTokenGroupMember(buffer: Buffer | Uint8Array): TokenGroupMember {
    const data = tokenGroupMemberCodec.decode(buffer);
    return {
        mint: new PublicKey(data.mint),
        group: new PublicKey(data.group),
        memberNumber: Buffer.from(data.memberNumber).readUInt32LE(),
    };
}

// Uint8Array(4) to number
export function u32ToNumber(buffer: Buffer): number {
    return buffer.readUInt32LE();
}
