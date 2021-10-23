import { struct, u8 } from '@solana/buffer-layout';
import { bool, publicKey } from '@solana/buffer-layout-utils';
import { Commitment, Connection, PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { TOKEN_PROGRAM_ID } from '../constants';
import { TokenError } from '../errors';

/** Information about a multisig */
export interface Multisig {
    /** The number of signers required */
    m: number;
    /** Number of possible signers, corresponds to the number of `signers` that are valid */
    n: number;
    /** Is this mint initialized */
    isInitialized: boolean;
    /** The full set of signers, of which `n` are nonempty */
    signer1: PublicKey;
    signer2: PublicKey;
    signer3: PublicKey;
    signer4: PublicKey;
    signer5: PublicKey;
    signer6: PublicKey;
    signer7: PublicKey;
    signer8: PublicKey;
    signer9: PublicKey;
    signer10: PublicKey;
    signer11: PublicKey;
}

/** @TODO: document */
export const MultisigLayout = struct<Multisig>([
    u8('m'),
    u8('n'),
    bool('isInitialized'),
    publicKey('signer1'),
    publicKey('signer2'),
    publicKey('signer3'),
    publicKey('signer4'),
    publicKey('signer5'),
    publicKey('signer6'),
    publicKey('signer7'),
    publicKey('signer8'),
    publicKey('signer9'),
    publicKey('signer10'),
    publicKey('signer11'),
]);

/** @TODO: document */
export const MULTISIG_LEN = MultisigLayout.span;

/** Get the minimum lamport balance for a Multsig to be rent exempt
 *
 * @param connection @TODO: docs
 * @param commitment @TODO: docs
 *
 * @return amount of lamports required
 */
export async function getMinimumBalanceForRentExemptMultisig(
    connection: Connection,
    commitment?: Commitment
): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(MULTISIG_LEN, commitment);
}

/**
 * Retrieve Multisig information
 *
 * @param multisig Public key of the account
 * @TODO: docs
 */
export async function getMultisigInfo(
    connection: Connection,
    multisig: PublicKey,
    programId = TOKEN_PROGRAM_ID
): Promise<Multisig> {
    const info = await connection.getAccountInfo(multisig);
    if (!info) throw new Error(TokenError.ACCOUNT_NOT_FOUND);
    if (!info.owner.equals(programId)) throw new Error(TokenError.INVALID_ACCOUNT_OWNER);
    if (info.data.length != MULTISIG_LEN) throw new Error(TokenError.INVALID_ACCOUNT_SIZE);

    return MultisigLayout.decode(Buffer.from(info.data));
}
