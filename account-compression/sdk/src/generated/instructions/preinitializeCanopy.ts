/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet';
import * as web3 from '@solana/web3.js';

/**
 * @category Instructions
 * @category PreinitializeCanopy
 * @category generated
 */
export type PreinitializeCanopyInstructionArgs = {
    canopyNodes: number[] /* size: 32 */[];
    maxBufferSize: number;
    maxDepth: number;
    startIndex: number;
};
/**
 * @category Instructions
 * @category PreinitializeCanopy
 * @category generated
 */
export const preinitializeCanopyStruct = new beet.FixableBeetArgsStruct<
    PreinitializeCanopyInstructionArgs & {
        instructionDiscriminator: number[] /* size: 8 */;
    }
>(
    [
        ['instructionDiscriminator', beet.uniformFixedSizeArray(beet.u8, 8)],
        ['maxDepth', beet.u32],
        ['maxBufferSize', beet.u32],
        ['startIndex', beet.u32],
        ['canopyNodes', beet.array(beet.uniformFixedSizeArray(beet.u8, 32))],
    ],
    'PreinitializeCanopyInstructionArgs',
);
/**
 * Accounts required by the _preinitializeCanopy_ instruction
 *
 * @property [_writable_] merkleTree
 * @property [**signer**] authority
 * @property [] noop
 * @category Instructions
 * @category PreinitializeCanopy
 * @category generated
 */
export type PreinitializeCanopyInstructionAccounts = {
    anchorRemainingAccounts?: web3.AccountMeta[];
    authority: web3.PublicKey;
    merkleTree: web3.PublicKey;
    noop: web3.PublicKey;
};

export const preinitializeCanopyInstructionDiscriminator = [233, 92, 157, 34, 63, 20, 168, 13];

/**
 * Creates a _PreinitializeCanopy_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @param args to provide as instruction data to the program
 *
 * @category Instructions
 * @category PreinitializeCanopy
 * @category generated
 */
export function createPreinitializeCanopyInstruction(
    accounts: PreinitializeCanopyInstructionAccounts,
    args: PreinitializeCanopyInstructionArgs,
    programId = new web3.PublicKey('cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK'),
) {
    const [data] = preinitializeCanopyStruct.serialize({
        instructionDiscriminator: preinitializeCanopyInstructionDiscriminator,
        ...args,
    });
    const keys: web3.AccountMeta[] = [
        {
            isSigner: false,
            isWritable: true,
            pubkey: accounts.merkleTree,
        },
        {
            isSigner: true,
            isWritable: false,
            pubkey: accounts.authority,
        },
        {
            isSigner: false,
            isWritable: false,
            pubkey: accounts.noop,
        },
    ];

    if (accounts.anchorRemainingAccounts != null) {
        for (const acc of accounts.anchorRemainingAccounts) {
            keys.push(acc);
        }
    }

    const ix = new web3.TransactionInstruction({
        data,
        keys,
        programId,
    });
    return ix;
}
