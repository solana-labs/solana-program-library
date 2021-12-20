import { struct, u8 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { addSigners } from './internal';
import { TokenInstruction } from './types';

/** Authority types defined by the program */
export enum AuthorityType {
    MintTokens = 0,
    FreezeAccount = 1,
    AccountOwner = 2,
    CloseAccount = 3,
}

/** TODO: docs */
export interface SetAuthorityInstructionData {
    instruction: TokenInstruction.SetAuthority;
    authorityType: AuthorityType;
    newAuthorityOption: 1 | 0;
    newAuthority: PublicKey;
}

/** TODO: docs */
export const setAuthorityInstructionDataStructure = struct<SetAuthorityInstructionData>([
    u8('instruction'),
    u8('authorityType'),
    u8('newAuthorityOption'),
    publicKey('newAuthority'),
]);

/**
 * Construct a SetAuthority instruction
 *
 * @param account          Address of the token account
 * @param newAuthority     New authority of the account
 * @param authorityType    Type of authority to set
 * @param currentAuthority Current authority of the specified type
 * @param multiSigners     Signing accounts if `currentAuthority` is a multisig
 * @param programId        SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createSetAuthorityInstruction(
    account: PublicKey,
    newAuthority: PublicKey | null,
    authorityType: AuthorityType,
    currentAuthority: PublicKey,
    multiSigners: Signer[] = [],
    programId = TOKEN_PROGRAM_ID
): TransactionInstruction {
    const keys = addSigners([{ pubkey: account, isSigner: false, isWritable: true }], currentAuthority, multiSigners);

    const data = Buffer.alloc(setAuthorityInstructionDataStructure.span);
    setAuthorityInstructionDataStructure.encode(
        {
            instruction: TokenInstruction.SetAuthority,
            authorityType,
            newAuthorityOption: newAuthority ? 1 : 0,
            newAuthority: newAuthority || new PublicKey(0),
        },
        data
    );

    return new TransactionInstruction({ keys, programId, data });
}
