import { struct, u8 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants.js';
import { TokenInvalidInstructionDataError, TokenInvalidInstructionKeysError, TokenInvalidInstructionProgramError, TokenInvalidInstructionTypeError, } from '../errors.js';
import { addSigners } from './internal.js';
import { TokenInstruction } from './types.js';
/** Authority types defined by the program */
export var AuthorityType;
(function (AuthorityType) {
    AuthorityType[AuthorityType["MintTokens"] = 0] = "MintTokens";
    AuthorityType[AuthorityType["FreezeAccount"] = 1] = "FreezeAccount";
    AuthorityType[AuthorityType["AccountOwner"] = 2] = "AccountOwner";
    AuthorityType[AuthorityType["CloseAccount"] = 3] = "CloseAccount";
})(AuthorityType || (AuthorityType = {}));
/** TODO: docs */
export const setAuthorityInstructionData = struct([
    u8('instruction'),
    u8('authorityType'),
    u8('newAuthorityOption'),
    publicKey('newAuthority'),
]);
/**
 * Construct a SetAuthority instruction
 *
 * @param account          Address of the token account
 * @param currentAuthority Current authority of the specified type
 * @param authorityType    Type of authority to set
 * @param newAuthority     New authority of the account
 * @param multiSigners     Signing accounts if `currentAuthority` is a multisig
 * @param programId        SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createSetAuthorityInstruction(account, currentAuthority, authorityType, newAuthority, multiSigners = [], programId = TOKEN_PROGRAM_ID) {
    const keys = addSigners([{ pubkey: account, isSigner: false, isWritable: true }], currentAuthority, multiSigners);
    const data = Buffer.alloc(setAuthorityInstructionData.span);
    setAuthorityInstructionData.encode({
        instruction: TokenInstruction.SetAuthority,
        authorityType,
        newAuthorityOption: newAuthority ? 1 : 0,
        newAuthority: newAuthority || new PublicKey(0),
    }, data);
    return new TransactionInstruction({ keys, programId, data });
}
/**
 * Decode a SetAuthority instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
export function decodeSetAuthorityInstruction(instruction, programId = TOKEN_PROGRAM_ID) {
    if (!instruction.programId.equals(programId))
        throw new TokenInvalidInstructionProgramError();
    if (instruction.data.length !== setAuthorityInstructionData.span)
        throw new TokenInvalidInstructionDataError();
    const { keys: { account, currentAuthority, multiSigners }, data, } = decodeSetAuthorityInstructionUnchecked(instruction);
    if (data.instruction !== TokenInstruction.SetAuthority)
        throw new TokenInvalidInstructionTypeError();
    if (!account || !currentAuthority)
        throw new TokenInvalidInstructionKeysError();
    // TODO: key checks?
    return {
        programId,
        keys: {
            account,
            currentAuthority,
            multiSigners,
        },
        data,
    };
}
/**
 * Decode a SetAuthority instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
export function decodeSetAuthorityInstructionUnchecked({ programId, keys: [account, currentAuthority, ...multiSigners], data, }) {
    const { instruction, authorityType, newAuthorityOption, newAuthority } = setAuthorityInstructionData.decode(data);
    return {
        programId,
        keys: {
            account,
            currentAuthority,
            multiSigners,
        },
        data: {
            instruction,
            authorityType,
            newAuthority: newAuthorityOption ? newAuthority : null,
        },
    };
}
//# sourceMappingURL=setAuthority.js.map