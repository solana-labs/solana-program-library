import { struct, u8 } from '@solana/buffer-layout';
import { publicKey, u64 } from '@solana/buffer-layout-utils';
import { AccountMeta, PublicKey, Signer, SYSVAR_RENT_PUBKEY, TransactionInstruction } from '@solana/web3.js';
import { AuthorityType } from './authority';
import { EMPTY_PUBLIC_KEY, TOKEN_PROGRAM_ID } from './constants';

export enum TokenInstruction {
    InitializeMint,
    InitializeAccount,
    InitializeMultisig,
    Transfer,
    Approve,
    Revoke,
    SetAuthority,
    MintTo,
    Burn,
    CloseAccount,
    FreezeAccount,
    ThawAccount,
    TransferChecked,
    ApproveChecked,
    MintToChecked,
    BurnChecked,
    InitializeAccount2, // @TODO: implement
    SyncNative,
    InitializeAccount3, // @TODO: implement
    InitializeMultisig2, // @TODO: implement
    InitializeMint2, // @TODO: implement
}

/**
 * Construct an InitializeMint instruction
 *
 * @param mint Token mint account
 * @param decimals Number of decimals in token account amounts
 * @param mintAuthority Minting authority
 * @param freezeAuthority Optional authority that can freeze token accounts
 * @param programId SPL Token program account
 */
export function createInitializeMintInstruction(
    mint: PublicKey,
    decimals: number,
    mintAuthority: PublicKey,
    freezeAuthority: PublicKey | null,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = [
        { pubkey: mint, isSigner: false, isWritable: true },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ];

    const dataLayout = struct<{
        instruction: TokenInstruction;
        decimals: number;
        mintAuthority: PublicKey;
        freezeAuthorityOption: 1 | 0;
        freezeAuthority: PublicKey;
    }>([
        u8('instruction'),
        u8('decimals'),
        publicKey('mintAuthority'),
        u8('freezeAuthorityOption'),
        publicKey('freezeAuthority'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.InitializeMint,
            decimals,
            mintAuthority,
            freezeAuthorityOption: freezeAuthority ? 1 : 0,
            freezeAuthority: freezeAuthority || EMPTY_PUBLIC_KEY,
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct an InitializeAccount instruction
 *
 * @param mint Token mint account
 * @param account New account
 * @param owner Owner of the new account
 * @param programId SPL Token program account
 */
export function createInitializeAccountInstruction(
    mint: PublicKey,
    account: PublicKey,
    owner: PublicKey,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = [
        { pubkey: account, isSigner: false, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: owner, isSigner: false, isWritable: false },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ];

    const dataLayout = struct<{ instruction: TokenInstruction }>([u8('instruction')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: TokenInstruction.InitializeAccount }, data);

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct an InitializeMultisig instruction
 *
 * @param account Multisig account
 * @param signers Full set of signers
 * @param m Number of required signatures
 * @param programId SPL Token program account
 */
export function createInitializeMultisigInstruction(
    account: PublicKey,
    signers: PublicKey[],
    m: number,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = [
        { pubkey: account, isSigner: false, isWritable: true },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ];
    for (const signer of signers) {
        keys.push({ pubkey: signer, isSigner: false, isWritable: false });
    }

    const dataLayout = struct<{
        instruction: TokenInstruction;
        m: number;
    }>([u8('instruction'), u8('m')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.InitializeMultisig,
            m,
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a Transfer instruction
 *
 * @param source Source account
 * @param destination Destination account
 * @param owner Owner of the source account
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param amount Number of tokens to transfer
 * @param programId SPL Token program account
 */
export function createTransferInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: source, isSigner: false, isWritable: true },
            { pubkey: destination, isSigner: false, isWritable: true },
        ],
        owner,
        multiSigners
    );

    const dataLayout = struct<{
        instruction: TokenInstruction;
        amount: bigint;
    }>([u8('instruction'), u64('amount')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.Transfer,
            amount: BigInt(amount),
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct an Approve instruction
 *
 * @param account Public key of the account
 * @param delegate Account authorized to perform a transfer of tokens from the source account
 * @param owner Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount Maximum number of tokens the delegate may transfer
 * @param programId SPL Token program account
 */
export function createApproveInstruction(
    account: PublicKey,
    delegate: PublicKey,
    owner: PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: delegate, isSigner: false, isWritable: false },
        ],
        owner,
        multiSigners
    );

    const dataLayout = struct<{
        instruction: TokenInstruction;
        amount: bigint;
    }>([u8('instruction'), u64('amount')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.Approve,
            amount: BigInt(amount),
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a Revoke instruction
 *
 * @param account Public key of the account
 * @param owner Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param programId SPL Token program account
 */
export function createRevokeInstruction(
    account: PublicKey,
    owner: PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners([{ pubkey: account, isSigner: false, isWritable: true }], owner, multiSigners);

    const dataLayout = struct<{ instruction: TokenInstruction }>([u8('instruction')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: TokenInstruction.Revoke }, data);

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a SetAuthority instruction
 *
 * @param account Public key of the account
 * @param newAuthority New authority of the account
 * @param authorityType Type of authority to set
 * @param currentAuthority Current authority of the specified type
 * @param multiSigners Signing accounts if `currentAuthority` is a multiSig
 * @param programId SPL Token program account
 */
export function createSetAuthorityInstruction(
    account: PublicKey,
    newAuthority: PublicKey | null,
    authorityType: AuthorityType,
    currentAuthority: PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners([{ pubkey: account, isSigner: false, isWritable: true }], currentAuthority, multiSigners);

    const dataLayout = struct<{
        instruction: TokenInstruction;
        authorityType: AuthorityType;
        newAuthorityOption: 1 | 0;
        newAuthority: PublicKey;
    }>([u8('instruction'), u8('authorityType'), u8('newAuthorityOption'), publicKey('newAuthority')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.SetAuthority,
            authorityType,
            newAuthorityOption: newAuthority ? 1 : 0,
            newAuthority: newAuthority || EMPTY_PUBLIC_KEY,
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a MintTo instruction
 *
 * @param mint Public key of the mint
 * @param dest Public key of the account to mint to
 * @param authority The mint authority
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param amount Amount to mint
 * @param programId SPL Token program account
 */
export function createMintToInstruction(
    mint: PublicKey,
    dest: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: mint, isSigner: false, isWritable: true },
            { pubkey: dest, isSigner: false, isWritable: true },
        ],
        authority,
        multiSigners
    );

    const dataLayout = struct<{
        instruction: TokenInstruction;
        amount: bigint;
    }>([u8('instruction'), u64('amount')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.MintTo,
            amount: BigInt(amount),
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a Burn instruction
 *
 * @param mint Mint for the account
 * @param account Account to burn tokens from
 * @param owner Owner of the account
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param amount amount to burn
 * @param programId SPL Token program account
 */
export function createBurnInstruction(
    mint: PublicKey,
    account: PublicKey,
    owner: PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: true },
        ],
        owner,
        multiSigners
    );

    const dataLayout = struct<{
        instruction: TokenInstruction;
        amount: bigint;
    }>([u8('instruction'), u64('amount')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.Burn,
            amount: BigInt(amount),
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a Close instruction
 *
 * @param account Account to close
 * @param dest Account to receive the remaining balance of the closed account
 * @param authority Account Close authority
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param programId SPL Token program account
 */
export function createCloseAccountInstruction(
    account: PublicKey,
    dest: PublicKey,
    owner: PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: dest, isSigner: false, isWritable: true },
        ],
        owner,
        multiSigners
    );

    const dataLayout = struct<{ instruction: TokenInstruction }>([u8('instruction')]);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: TokenInstruction.CloseAccount }, data);

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a Freeze instruction
 *
 * @param account Account to freeze
 * @param mint Mint account
 * @param authority Mint freeze authority
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param programId SPL Token program account
 */
export function createFreezeAccountInstruction(
    account: PublicKey,
    mint: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
        ],
        authority,
        multiSigners
    );

    const dataLayout = struct<{ instruction: TokenInstruction }>([u8('instruction')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: TokenInstruction.FreezeAccount }, data);

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a Thaw instruction
 *
 * @param account Account to thaw
 * @param mint Mint account
 * @param authority Mint freeze authority
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param programId SPL Token program account
 */
export function createThawAccountInstruction(
    account: PublicKey,
    mint: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
        ],
        authority,
        multiSigners
    );

    const dataLayout = struct<{ instruction: TokenInstruction }>([u8('instruction')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: TokenInstruction.ThawAccount }, data);

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a TransferChecked instruction
 *
 * @param source Source account
 * @param mint Mint account
 * @param destination Destination account
 * @param owner Owner of the source account
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param amount Number of tokens to transfer
 * @param decimals Number of decimals in transfer amount
 * @param programId SPL Token program account
 */
export function createTransferCheckedInstruction(
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: source, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
            { pubkey: destination, isSigner: false, isWritable: true },
        ],
        owner,
        multiSigners
    );

    const dataLayout = struct<{
        instruction: TokenInstruction;
        amount: bigint;
        decimals: number;
    }>([u8('instruction'), u64('amount'), u8('decimals')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.TransferChecked,
            amount: BigInt(amount),
            decimals,
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct an ApproveChecked instruction
 *
 * @param account Public key of the account
 * @param mint Mint account
 * @param delegate Account authorized to perform a transfer of tokens from the source account
 * @param owner Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount Maximum number of tokens the delegate may transfer
 * @param decimals Number of decimals in approve amount
 * @param programId SPL Token program account
 */
export function createApproveCheckedInstruction(
    account: PublicKey,
    mint: PublicKey,
    delegate: PublicKey,
    owner: PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: false },
            { pubkey: delegate, isSigner: false, isWritable: false },
        ],
        owner,
        multiSigners
    );

    const dataLayout = struct<{
        instruction: TokenInstruction;
        amount: bigint;
        decimals: number;
    }>([u8('instruction'), u64('amount'), u8('decimals')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.ApproveChecked,
            amount: BigInt(amount),
            decimals,
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a MintToChecked instruction
 *
 * @param mint Public key of the mint
 * @param dest Public key of the account to mint to
 * @param authority The mint authority
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param amount Amount to mint
 * @param decimals Number of decimals in amount to mint
 * @param programId SPL Token program account
 */
export function createMintToCheckedInstruction(
    mint: PublicKey,
    dest: PublicKey,
    authority: PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: mint, isSigner: false, isWritable: true },
            { pubkey: dest, isSigner: false, isWritable: true },
        ],
        authority,
        multiSigners
    );

    const dataLayout = struct<{
        instruction: TokenInstruction;
        amount: bigint;
        decimals: number;
    }>([u8('instruction'), u64('amount'), u8('decimals')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.MintToChecked,
            amount: BigInt(amount),
            decimals,
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a BurnChecked instruction
 *
 * @param mint Mint for the account
 * @param account Account to burn tokens from
 * @param owner Owner of the account
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param amount amount to burn
 * @param programId SPL Token program account
 */
export function createBurnCheckedInstruction(
    mint: PublicKey,
    account: PublicKey,
    owner: PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners(
        [
            { pubkey: account, isSigner: false, isWritable: true },
            { pubkey: mint, isSigner: false, isWritable: true },
        ],
        owner,
        multiSigners
    );

    const dataLayout = struct<{
        instruction: TokenInstruction;
        amount: bigint;
        decimals: number;
    }>([u8('instruction'), u64('amount'), u8('decimals')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
        {
            instruction: TokenInstruction.BurnChecked,
            amount: BigInt(amount),
            decimals,
        },
        data
    );

    return new TransactionInstruction({
        keys,
        programId,
        data,
    });
}

/**
 * Construct a SyncNative instruction
 *
 * @param nativeAccount Account to sync lamports from
 * @param programId SPL Token program account
 */
export function createSyncNativeInstruction(
    nativeAccount: PublicKey,
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = [{ pubkey: nativeAccount, isSigner: false, isWritable: true }];

    const dataLayout = struct<{ instruction: TokenInstruction }>([u8('instruction')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode({ instruction: TokenInstruction.SyncNative }, data);

    return new TransactionInstruction({ keys, programId, data });
}

function addSigners(keys: AccountMeta[], pubkey: PublicKey, multiSigners: Signer[]): AccountMeta[] {
    if (multiSigners.length) {
        keys.push({ pubkey, isSigner: false, isWritable: false });
        for (const signer of multiSigners) {
            keys.push({ pubkey: signer.publicKey, isSigner: true, isWritable: false });
        }
    } else {
        keys.push({ pubkey, isSigner: true, isWritable: false });
    }
    return keys;
}
