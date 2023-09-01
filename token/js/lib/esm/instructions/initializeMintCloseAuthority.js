import { struct, u8 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { programSupportsExtensions } from '../constants.js';
import { TokenInvalidInstructionDataError, TokenInvalidInstructionKeysError, TokenInvalidInstructionProgramError, TokenInvalidInstructionTypeError, TokenUnsupportedInstructionError, } from '../errors.js';
import { TokenInstruction } from './types.js';
/** TODO: docs */
export const initializeMintCloseAuthorityInstructionData = struct([
    u8('instruction'),
    u8('closeAuthorityOption'),
    publicKey('closeAuthority'),
]);
/**
 * Construct an InitializeMintCloseAuthority instruction
 *
 * @param mint            Token mint account
 * @param closeAuthority  Optional authority that can close the mint
 * @param programId       SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
export function createInitializeMintCloseAuthorityInstruction(mint, closeAuthority, programId) {
    if (!programSupportsExtensions(programId)) {
        throw new TokenUnsupportedInstructionError();
    }
    const keys = [{ pubkey: mint, isSigner: false, isWritable: true }];
    const data = Buffer.alloc(initializeMintCloseAuthorityInstructionData.span);
    initializeMintCloseAuthorityInstructionData.encode({
        instruction: TokenInstruction.InitializeMintCloseAuthority,
        closeAuthorityOption: closeAuthority ? 1 : 0,
        closeAuthority: closeAuthority || new PublicKey(0),
    }, data);
    return new TransactionInstruction({ keys, programId, data });
}
/**
 * Decode an InitializeMintCloseAuthority instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
export function decodeInitializeMintCloseAuthorityInstruction(instruction, programId) {
    if (!instruction.programId.equals(programId))
        throw new TokenInvalidInstructionProgramError();
    if (instruction.data.length !== initializeMintCloseAuthorityInstructionData.span)
        throw new TokenInvalidInstructionDataError();
    const { keys: { mint }, data, } = decodeInitializeMintCloseAuthorityInstructionUnchecked(instruction);
    if (data.instruction !== TokenInstruction.InitializeMintCloseAuthority)
        throw new TokenInvalidInstructionTypeError();
    if (!mint)
        throw new TokenInvalidInstructionKeysError();
    return {
        programId,
        keys: {
            mint,
        },
        data,
    };
}
/**
 * Decode an InitializeMintCloseAuthority instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
export function decodeInitializeMintCloseAuthorityInstructionUnchecked({ programId, keys: [mint], data, }) {
    const { instruction, closeAuthorityOption, closeAuthority } = initializeMintCloseAuthorityInstructionData.decode(data);
    return {
        programId,
        keys: {
            mint,
        },
        data: {
            instruction,
            closeAuthority: closeAuthorityOption ? closeAuthority : null,
        },
    };
}
//# sourceMappingURL=initializeMintCloseAuthority.js.map