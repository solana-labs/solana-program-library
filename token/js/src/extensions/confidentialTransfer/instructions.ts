import { struct, u8 } from '@solana/buffer-layout';
import { bool, publicKey } from '@solana/buffer-layout-utils';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { programSupportsExtensions, TOKEN_2022_PROGRAM_ID } from '../../constants.js';
import { TokenUnsupportedInstructionError } from '../../errors.js';
import { TokenInstruction } from '../../instructions/types.js';
import { PodElGamalPubkey } from 'solana-zk-token-sdk-experimental';
import { elgamalPublicKey } from './elgamal.js';

export enum ConfidentialTransferInstruction {
    InitializeMint = 0,
}

export interface InitializeMintData {
    instruction: TokenInstruction.ConfidentialTransferExtension;
    confidentialTransferInstruction: ConfidentialTransferInstruction.InitializeMint;
    confidentialTransferMintAuthority: PublicKey | null;
    autoApproveNewAccounts: boolean;
    auditorElGamalPubkey: PodElGamalPubkey | null;
}

export const initializeMintData = struct<InitializeMintData>([
    u8('instruction'),
    u8('confidentialTransferInstruction'),
    publicKey('confidentialTransferMintAuthority'),
    bool('autoApproveNewAccounts'),
    elgamalPublicKey('auditorElGamalPubkey'),
]);

export function createConfidentialTransferInitializeMintInstruction(
    mint: PublicKey,
    confidentialTransferMintAuthority: PublicKey | null,
    autoApproveNewAccounts: boolean,
    auditorElGamalPubkey: PodElGamalPubkey | null,
    programId = TOKEN_2022_PROGRAM_ID
): TransactionInstruction {
    if (!programSupportsExtensions(programId)) {
        throw new TokenUnsupportedInstructionError();
    }
    const keys = [{ pubkey: mint, isSigner: false, isWritable: true }];

    const data = Buffer.alloc(initializeMintData.span);

    initializeMintData.encode(
        {
            instruction: TokenInstruction.ConfidentialTransferExtension,
            confidentialTransferInstruction: ConfidentialTransferInstruction.InitializeMint,
            confidentialTransferMintAuthority: confidentialTransferMintAuthority ?? PublicKey.default,
            autoApproveNewAccounts: autoApproveNewAccounts,
            auditorElGamalPubkey: auditorElGamalPubkey ?? PodElGamalPubkey.default(),
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}
