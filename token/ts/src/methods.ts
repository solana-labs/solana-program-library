import {
    Connection,
    Keypair,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    SystemProgram,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { ACCOUNT_LEN } from './account';
import { AuthorityType } from './authority';
import { NATIVE_MINT, TOKEN_PROGRAM_ID } from './constants';
import {
    createApproveCheckedInstruction,
    createApproveInstruction,
    createBurnCheckedInstruction,
    createBurnInstruction,
    createCloseAccountInstruction,
    createFreezeAccountInstruction,
    createInitializeAccountInstruction,
    createInitializeMintInstruction,
    createInitializeMultisigInstruction,
    createMintToCheckedInstruction,
    createMintToInstruction,
    createRevokeInstruction,
    createSetAuthorityInstruction,
    createSyncNativeInstruction,
    createThawAccountInstruction,
    createTransferCheckedInstruction,
    createTransferInstruction,
} from './instructions';
import { MINT_LEN } from './mint';
import { MULTISIG_LEN } from './multisig';
import {
    getMinimumBalanceForRentExemptAccount,
    getMinimumBalanceForRentExemptMint,
    getMinimumBalanceForRentExemptMultisig,
} from './rent';

/**
 * Create and initialize a token.
 *
 * @param connection The connection to use
 * @param payer Fee payer for transaction
 * @param mintAuthority Account or multisig that will control minting
 * @param freezeAuthority Optional account or multisig that can freeze token accounts
 * @param decimals Location of the decimal place
 * @param programId Optional token programId, uses the system programId by default
 * @return Token object for the newly minted token
 */
export async function createMint(
    connection: Connection,
    payer: Signer,
    mintAuthority: PublicKey,
    freezeAuthority: PublicKey | null,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    const lamports = await getMinimumBalanceForRentExemptMint(connection);

    const mintAccount = Keypair.generate();

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: mintAccount.publicKey,
            space: MINT_LEN,
            lamports,
            programId,
        }),
        createInitializeMintInstruction(mintAccount.publicKey, decimals, mintAuthority, freezeAuthority, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, mintAccount]);
}

/**
 * Create and initialize a new account.
 *
 * This account may then be used as a `transfer()` or `approve()` destination
 *
 * @param owner User account that will own the new account
 * @return Public key of the new empty account
 */
export async function createAccount(
    connection: Connection,
    mint: PublicKey,
    owner: PublicKey,
    payer: Signer,
    programId = TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    const lamports = await getMinimumBalanceForRentExemptAccount(connection);

    const newAccount = Keypair.generate();

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: newAccount.publicKey,
            space: ACCOUNT_LEN,
            lamports,
            programId,
        }),
        createInitializeAccountInstruction(mint, newAccount.publicKey, owner, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, newAccount]);

    return newAccount.publicKey;
}

/**
 * Create and initialize a new account on the special native token mint.
 *
 * In order to be wrapped, the account must have a balance of native tokens
 * when it is initialized with the token program.
 *
 * This function sends lamports to the new account before initializing it.
 *
 * @param connection A solana web3 connection
 * @param owner The owner of the new token account
 * @param payer The source of the lamports to initialize, and payer of the initialization fees
 * @param amount The amount of lamports to wrap
 * @param programId The token program ID
 * @return {Promise<PublicKey>} The new token account
 */
export async function createWrappedNativeAccount(
    connection: Connection,
    owner: PublicKey,
    payer: Signer,
    amount: number,
    programId = TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    const lamports = await getMinimumBalanceForRentExemptAccount(connection);

    const newAccount = Keypair.generate();

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: newAccount.publicKey,
            space: ACCOUNT_LEN,
            lamports,
            programId,
        }),
        SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: newAccount.publicKey,
            lamports: amount,
        }),
        createInitializeAccountInstruction(NATIVE_MINT, newAccount.publicKey, owner, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, newAccount]);

    return newAccount.publicKey;
}

/**
 * Create and initialize a new multisig.
 *
 * This account may then be used for multisignature verification
 *
 * @param connection A solana web3 connection
 * @param payer Payer of the initialization fees
 * @param m Number of required signatures
 * @param multiSigners Full set of signers
 * @return Public key of the new multisig account
 */
export async function createMultisig(
    connection: Connection,
    payer: Signer,
    m: number,
    multiSigners: PublicKey[],
    programId = TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    const lamports = await getMinimumBalanceForRentExemptMultisig(connection);

    const newAccount = Keypair.generate();

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: newAccount.publicKey,
            space: MULTISIG_LEN,
            lamports,
            programId,
        }),
        createInitializeMultisigInstruction(newAccount.publicKey, multiSigners, m, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, newAccount]);

    return newAccount.publicKey;
}

/**
 * Transfer tokens to another account
 *
 * @param source Source account
 * @param destination Destination account
 * @param owner Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount Number of tokens to transfer
 */
export async function transfer(
    connection: Connection,
    payer: Signer,
    source: PublicKey,
    destination: PublicKey,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let ownerPublicKey: PublicKey;
    let signers: Signer[];
    if (owner instanceof PublicKey) {
        ownerPublicKey = owner;
        signers = multiSigners;
    } else {
        ownerPublicKey = owner.publicKey;
        signers = [owner];
    }

    const transaction = new Transaction().add(
        createTransferInstruction(source, destination, ownerPublicKey, multiSigners, amount, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Grant a third-party permission to transfer up the specified number of tokens from an account
 *
 * @param account Public key of the account
 * @param delegate Account authorized to perform a transfer tokens from the source account
 * @param owner Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount Maximum number of tokens the delegate may transfer
 */
export async function approve(
    connection: Connection,
    payer: Signer,
    account: PublicKey,
    delegate: PublicKey,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let ownerPublicKey: PublicKey;
    let signers: Signer[];
    if (owner instanceof PublicKey) {
        ownerPublicKey = owner;
        signers = multiSigners;
    } else {
        ownerPublicKey = owner.publicKey;
        signers = [owner];
    }

    const transaction = new Transaction().add(
        createApproveInstruction(account, delegate, ownerPublicKey, multiSigners, amount, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Remove approval for the transfer of any remaining tokens
 *
 * @param account Public key of the account
 * @param owner Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 */
export async function revoke(
    connection: Connection,
    payer: Signer,
    account: PublicKey,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let ownerPublicKey: PublicKey;
    let signers: Signer[];
    if (owner instanceof PublicKey) {
        ownerPublicKey = owner;
        signers = multiSigners;
    } else {
        ownerPublicKey = owner.publicKey;
        signers = [owner];
    }

    const transaction = new Transaction().add(
        createRevokeInstruction(account, ownerPublicKey, multiSigners, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Assign a new authority to the account
 *
 * @param account Public key of the account
 * @param newAuthority New authority of the account
 * @param authorityType Type of authority to set
 * @param currentAuthority Current authority of the account
 * @param multiSigners Signing accounts if `currentAuthority` is a multiSig
 */
export async function setAuthority(
    connection: Connection,
    payer: Signer,
    account: PublicKey,
    newAuthority: PublicKey | null,
    authorityType: AuthorityType,
    currentAuthority: Signer | PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let currentAuthorityPublicKey: PublicKey;
    let signers: Signer[];
    if (currentAuthority instanceof PublicKey) {
        currentAuthorityPublicKey = currentAuthority;
        signers = multiSigners;
    } else {
        currentAuthorityPublicKey = currentAuthority.publicKey;
        signers = [currentAuthority];
    }

    const transaction = new Transaction().add(
        createSetAuthorityInstruction(
            account,
            newAuthority,
            authorityType,
            currentAuthorityPublicKey,
            multiSigners,
            programId
        )
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Mint new tokens
 *
 * @param dest Public key of the account to mint to
 * @param authority Minting authority
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param amount Amount to mint
 */
export async function mintTo(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    dest: PublicKey,
    authority: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let authorityPublicKey: PublicKey;
    let signers: Signer[];
    if (authority instanceof PublicKey) {
        authorityPublicKey = authority;
        signers = multiSigners;
    } else {
        authorityPublicKey = authority.publicKey;
        signers = [authority];
    }

    const transaction = new Transaction().add(
        createMintToInstruction(mint, dest, authorityPublicKey, multiSigners, amount, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Burn tokens
 *
 * @param account Account to burn tokens from
 * @param owner Account owner
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount Amount to burn
 */
export async function burn(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    account: PublicKey,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let ownerPublicKey: PublicKey;
    let signers: Signer[];
    if (owner instanceof PublicKey) {
        ownerPublicKey = owner;
        signers = multiSigners;
    } else {
        ownerPublicKey = owner.publicKey;
        signers = [owner];
    }

    const transaction = new Transaction().add(
        createBurnInstruction(mint, account, ownerPublicKey, multiSigners, amount, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Close account
 *
 * @param account Account to close
 * @param dest Account to receive the remaining balance of the closed account
 * @param authority Authority which is allowed to close the account
 * @param multiSigners Signing accounts if `authority` is a multiSig
 */
export async function closeAccount(
    connection: Connection,
    payer: Signer,
    account: PublicKey,
    dest: PublicKey,
    authority: Signer | PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let authorityPublicKey;
    let signers: Signer[];
    if (authority instanceof PublicKey) {
        authorityPublicKey = authority;
        signers = multiSigners;
    } else {
        authorityPublicKey = authority.publicKey;
        signers = [authority];
    }

    const transaction = new Transaction().add(
        createCloseAccountInstruction(account, dest, authorityPublicKey, multiSigners, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Freeze account
 *
 * @param account Account to freeze
 * @param authority The mint freeze authority
 * @param multiSigners Signing accounts if `authority` is a multiSig
 */
export async function freezeAccount(
    connection: Connection,
    mint: PublicKey,
    payer: Signer,
    account: PublicKey,
    authority: Signer | PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let authorityPublicKey;
    let signers: Signer[];
    if (authority instanceof PublicKey) {
        authorityPublicKey = authority;
        signers = multiSigners;
    } else {
        authorityPublicKey = authority.publicKey;
        signers = [authority];
    }

    const transaction = new Transaction().add(
        createFreezeAccountInstruction(account, mint, authorityPublicKey, multiSigners, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Thaw account
 *
 * @param account Account to thaw
 * @param authority The mint freeze authority
 * @param multiSigners Signing accounts if `authority` is a multiSig
 */
export async function thawAccount(
    connection: Connection,
    mint: PublicKey,
    payer: Signer,
    account: PublicKey,
    authority: Signer | PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let authorityPublicKey;
    let signers: Signer[];
    if (authority instanceof PublicKey) {
        authorityPublicKey = authority;
        signers = multiSigners;
    } else {
        authorityPublicKey = authority.publicKey;
        signers = [authority];
    }

    const transaction = new Transaction().add(
        createThawAccountInstruction(account, mint, authorityPublicKey, multiSigners, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Transfer tokens to another account, asserting the token mint and decimals
 *
 * @param source Source account
 * @param destination Destination account
 * @param owner Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount Number of tokens to transfer
 * @param decimals Number of decimals in transfer amount
 */
export async function transferChecked(
    connection: Connection,
    mint: PublicKey,
    payer: Signer,
    source: PublicKey,
    destination: PublicKey,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let ownerPublicKey: PublicKey;
    let signers: Signer[];
    if (owner instanceof PublicKey) {
        ownerPublicKey = owner;
        signers = multiSigners;
    } else {
        ownerPublicKey = owner.publicKey;
        signers = [owner];
    }

    const transaction = new Transaction().add(
        createTransferCheckedInstruction(
            source,
            mint,
            destination,
            ownerPublicKey,
            multiSigners,
            amount,
            decimals,
            programId
        )
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Grant a third-party permission to transfer up the specified number of tokens from an account,
 * asserting the token mint and decimals
 *
 * @param account Public key of the account
 * @param delegate Account authorized to perform a transfer tokens from the source account
 * @param owner Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount Maximum number of tokens the delegate may transfer
 * @param decimals Number of decimals in approve amount
 */
export async function approveChecked(
    connection: Connection,
    mint: PublicKey,
    payer: Signer,
    account: PublicKey,
    delegate: PublicKey,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let ownerPublicKey: PublicKey;
    let signers: Signer[];
    if (owner instanceof PublicKey) {
        ownerPublicKey = owner;
        signers = multiSigners;
    } else {
        ownerPublicKey = owner.publicKey;
        signers = [owner];
    }

    const transaction = new Transaction().add(
        createApproveCheckedInstruction(
            account,
            mint,
            delegate,
            ownerPublicKey,
            multiSigners,
            amount,
            decimals,
            programId
        )
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Mint new tokens, asserting the token mint and decimals
 *
 * @param dest Public key of the account to mint to
 * @param authority Minting authority
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param amount Amount to mint
 * @param decimals Number of decimals in amount to mint
 */
export async function mintToChecked(
    connection: Connection,
    mint: PublicKey,
    payer: Signer,
    dest: PublicKey,
    authority: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let authorityPublicKey: PublicKey;
    let signers: Signer[];
    if (authority instanceof PublicKey) {
        authorityPublicKey = authority;
        signers = multiSigners;
    } else {
        authorityPublicKey = authority.publicKey;
        signers = [authority];
    }

    const transaction = new Transaction().add(
        createMintToCheckedInstruction(mint, dest, authorityPublicKey, multiSigners, amount, decimals, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Burn tokens, asserting the token mint and decimals
 *
 * @param account Account to burn tokens from
 * @param owner Account owner
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount Amount to burn
 * @param decimals Number of decimals in amount to burn
 */
export async function burnChecked(
    connection: Connection,
    mint: PublicKey,
    account: PublicKey,
    payer: Signer,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let ownerPublicKey: PublicKey;
    let signers: Signer[];
    if (owner instanceof PublicKey) {
        ownerPublicKey = owner;
        signers = multiSigners;
    } else {
        ownerPublicKey = owner.publicKey;
        signers = [owner];
    }

    const transaction = new Transaction().add(
        createBurnCheckedInstruction(mint, account, ownerPublicKey, multiSigners, amount, decimals, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}

/**
 * Sync amount in native SPL token account to underlying lamports
 *
 * @param nativeAccount Account to sync
 */
export async function syncNative(
    connection: Connection,
    nativeAccount: PublicKey,
    payer: Signer,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    const transaction = new Transaction().add(createSyncNativeInstruction(nativeAccount, programId));

    return await sendAndConfirmTransaction(connection, transaction, [payer]);
}
