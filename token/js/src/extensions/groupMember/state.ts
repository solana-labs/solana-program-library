import { struct, u32 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import { PublicKey } from '@solana/web3.js';
import type { TokenGroupMember } from '@solana/spl-token-group';
import type { Mint } from '../../state/mint.js';
import { ExtensionType, getExtensionData } from '../extensionType.js';

/** Buffer layout for de/serializing a TokenGroupMember extension */
export const TokenGroupMemberLayout = struct<TokenGroupMember>([
    publicKey('mint'),
    publicKey('group'),
    u32('memberNumber'),
]);

export const TOKEN_GROUP_MEMBER_SIZE = TokenGroupMemberLayout.span;

export function getTokenGroupMemberState(mint: Mint): Partial<TokenGroupMember> | null {
    const extensionData = getExtensionData(ExtensionType.TokenGroupMember, mint.tlvData);
    if (extensionData !== null) {
        const { mint, group, memberNumber } = TokenGroupMemberLayout.decode(extensionData);

        return {
            mint,
            group,
            memberNumber,
        };
    } else {
        return null;
    }
}
