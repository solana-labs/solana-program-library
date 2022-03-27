import { u8 } from '@solana/buffer-layout';
import { TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { TokenInvalidInstructionDataError, TokenInvalidInstructionTypeError } from '../errors';
import { decodeApproveInstruction, DecodedApproveInstruction } from './approve';
import { decodeApproveCheckedInstruction, DecodedApproveCheckedInstruction } from './approveChecked';
import { decodeBurnInstruction, DecodedBurnInstruction } from './burn';
import { decodeBurnCheckedInstruction, DecodedBurnCheckedInstruction } from './burnChecked';
import { decodeCloseAccountInstruction, DecodedCloseAccountInstruction } from './closeAccount';
import { DecodedFreezeAccountInstruction, decodeFreezeAccountInstruction } from './freezeAccount';
import { DecodedInitializeAccountInstruction, decodeInitializeAccountInstruction } from './initializeAccount';
import { DecodedInitializeMintInstruction, decodeInitializeMintInstruction } from './initializeMint';
import { DecodedInitializeMultisigInstruction, decodeInitializeMultisigInstruction } from './initializeMultisig';
import { DecodedMintToInstruction, decodeMintToInstruction } from './mintTo';
import { DecodedMintToCheckedInstruction, decodeMintToCheckedInstruction } from './mintToChecked';
import { DecodedRevokeInstruction, decodeRevokeInstruction } from './revoke';
import { DecodedSetAuthorityInstruction, decodeSetAuthorityInstruction } from './setAuthority';
import { DecodedSyncNativeInstruction, decodeSyncNativeInstruction } from './syncNative';
import { DecodedThawAccountInstruction, decodeThawAccountInstruction } from './thawAccount';
import { DecodedTransferInstruction, decodeTransferInstruction } from './transfer';
import { DecodedTransferCheckedInstruction, decodeTransferCheckedInstruction } from './transferChecked';
import { TokenInstruction } from './types';

/** TODO: docs */
export type DecodedInstruction =
    | DecodedInitializeMintInstruction
    | DecodedInitializeAccountInstruction
    | DecodedInitializeMultisigInstruction
    | DecodedTransferInstruction
    | DecodedApproveInstruction
    | DecodedRevokeInstruction
    | DecodedSetAuthorityInstruction
    | DecodedMintToInstruction
    | DecodedBurnInstruction
    | DecodedCloseAccountInstruction
    | DecodedFreezeAccountInstruction
    | DecodedThawAccountInstruction
    | DecodedTransferCheckedInstruction
    | DecodedApproveCheckedInstruction
    | DecodedMintToCheckedInstruction
    | DecodedBurnCheckedInstruction
    // | DecodedInitializeAccount2Instruction
    | DecodedSyncNativeInstruction
    // | DecodedInitializeAccount3Instruction
    // | DecodedInitializeMultisig2Instruction
    // | DecodedInitializeMint2Instruction
    // TODO: implement ^ and remove `never`
    | never;

/** TODO: docs */
export function decodeInstruction(
    instruction: TransactionInstruction,
    programId = TOKEN_PROGRAM_ID
): DecodedInstruction {
    if (!instruction.data.length) throw new TokenInvalidInstructionDataError();

    const type = u8().decode(instruction.data);
    if (type === TokenInstruction.InitializeMint) return decodeInitializeMintInstruction(instruction, programId);
    if (type === TokenInstruction.InitializeAccount) return decodeInitializeAccountInstruction(instruction, programId);
    if (type === TokenInstruction.InitializeMultisig)
        return decodeInitializeMultisigInstruction(instruction, programId);
    if (type === TokenInstruction.Transfer) return decodeTransferInstruction(instruction, programId);
    if (type === TokenInstruction.Approve) return decodeApproveInstruction(instruction, programId);
    if (type === TokenInstruction.Revoke) return decodeRevokeInstruction(instruction, programId);
    if (type === TokenInstruction.SetAuthority) return decodeSetAuthorityInstruction(instruction, programId);
    if (type === TokenInstruction.MintTo) return decodeMintToInstruction(instruction, programId);
    if (type === TokenInstruction.Burn) return decodeBurnInstruction(instruction, programId);
    if (type === TokenInstruction.CloseAccount) return decodeCloseAccountInstruction(instruction, programId);
    if (type === TokenInstruction.FreezeAccount) return decodeFreezeAccountInstruction(instruction, programId);
    if (type === TokenInstruction.ThawAccount) return decodeThawAccountInstruction(instruction, programId);
    if (type === TokenInstruction.TransferChecked) return decodeTransferCheckedInstruction(instruction, programId);
    if (type === TokenInstruction.ApproveChecked) return decodeApproveCheckedInstruction(instruction, programId);
    if (type === TokenInstruction.MintToChecked) return decodeMintToCheckedInstruction(instruction, programId);
    if (type === TokenInstruction.BurnChecked) return decodeBurnCheckedInstruction(instruction, programId);
    // TODO: implement
    if (type === TokenInstruction.InitializeAccount2) throw new TokenInvalidInstructionTypeError();
    if (type === TokenInstruction.SyncNative) return decodeSyncNativeInstruction(instruction, programId);
    // TODO: implement
    if (type === TokenInstruction.InitializeAccount3) throw new TokenInvalidInstructionTypeError();
    // TODO: implement
    if (type === TokenInstruction.InitializeMultisig2) throw new TokenInvalidInstructionTypeError();
    // TODO: implement
    if (type === TokenInstruction.InitializeMint2) throw new TokenInvalidInstructionTypeError();

    throw new TokenInvalidInstructionTypeError();
}

/** TODO: docs */
export function isInitializeMintInstruction(decoded: DecodedInstruction): decoded is DecodedInitializeMintInstruction {
    return decoded.data.instruction === TokenInstruction.InitializeMint;
}

/** TODO: docs */
export function isInitializeAccountInstruction(
    decoded: DecodedInstruction
): decoded is DecodedInitializeAccountInstruction {
    return decoded.data.instruction === TokenInstruction.InitializeAccount;
}

/** TODO: docs */
export function isInitializeMultisigInstruction(
    decoded: DecodedInstruction
): decoded is DecodedInitializeMultisigInstruction {
    return decoded.data.instruction === TokenInstruction.InitializeMultisig;
}

/** TODO: docs */
export function isTransferInstruction(decoded: DecodedInstruction): decoded is DecodedTransferInstruction {
    return decoded.data.instruction === TokenInstruction.Transfer;
}

/** TODO: docs */
export function isApproveInstruction(decoded: DecodedInstruction): decoded is DecodedApproveInstruction {
    return decoded.data.instruction === TokenInstruction.Approve;
}

/** TODO: docs */
export function isRevokeInstruction(decoded: DecodedInstruction): decoded is DecodedRevokeInstruction {
    return decoded.data.instruction === TokenInstruction.Revoke;
}

/** TODO: docs */
export function isSetAuthorityInstruction(decoded: DecodedInstruction): decoded is DecodedSetAuthorityInstruction {
    return decoded.data.instruction === TokenInstruction.SetAuthority;
}

/** TODO: docs */
export function isMintToInstruction(decoded: DecodedInstruction): decoded is DecodedMintToInstruction {
    return decoded.data.instruction === TokenInstruction.MintTo;
}

/** TODO: docs */
export function isBurnInstruction(decoded: DecodedInstruction): decoded is DecodedBurnInstruction {
    return decoded.data.instruction === TokenInstruction.Burn;
}

/** TODO: docs */
export function isCloseAccountInstruction(decoded: DecodedInstruction): decoded is DecodedCloseAccountInstruction {
    return decoded.data.instruction === TokenInstruction.CloseAccount;
}

/** TODO: docs */
export function isFreezeAccountInstruction(decoded: DecodedInstruction): decoded is DecodedFreezeAccountInstruction {
    return decoded.data.instruction === TokenInstruction.FreezeAccount;
}

/** TODO: docs */
export function isThawAccountInstruction(decoded: DecodedInstruction): decoded is DecodedThawAccountInstruction {
    return decoded.data.instruction === TokenInstruction.ThawAccount;
}

/** TODO: docs */
export function isTransferCheckedInstruction(
    decoded: DecodedInstruction
): decoded is DecodedTransferCheckedInstruction {
    return decoded.data.instruction === TokenInstruction.TransferChecked;
}

/** TODO: docs */
export function isApproveCheckedInstruction(decoded: DecodedInstruction): decoded is DecodedApproveCheckedInstruction {
    return decoded.data.instruction === TokenInstruction.ApproveChecked;
}

/** TODO: docs */
export function isMintToCheckedInstruction(decoded: DecodedInstruction): decoded is DecodedMintToCheckedInstruction {
    return decoded.data.instruction === TokenInstruction.MintToChecked;
}

/** TODO: docs */
export function isBurnCheckedInstruction(decoded: DecodedInstruction): decoded is DecodedBurnCheckedInstruction {
    return decoded.data.instruction === TokenInstruction.BurnChecked;
}

/** TODO: docs, implement */
// export function isInitializeAccount2Instruction(
//     decoded: DecodedInstruction
// ): decoded is DecodedInitializeAccount2Instruction {
//     return decoded.data.instruction === TokenInstruction.InitializeAccount2;
// }

/** TODO: docs */
export function isSyncNativeInstruction(decoded: DecodedInstruction): decoded is DecodedSyncNativeInstruction {
    return decoded.data.instruction === TokenInstruction.SyncNative;
}

/** TODO: docs, implement */
// export function isInitializeAccount3Instruction(
//     decoded: DecodedInstruction
// ): decoded is DecodedInitializeAccount3Instruction {
//     return decoded.data.instruction === TokenInstruction.InitializeAccount3;
// }

/** TODO: docs, implement */
// export function isInitializeMultisig2Instruction(
//     decoded: DecodedInstruction
// ): decoded is DecodedInitializeMultisig2Instruction {
//     return decoded.data.instruction === TokenInstruction.InitializeMultisig2;
// }

/** TODO: docs, implement */
// export function isInitializeMint2Instruction(
//     decoded: DecodedInstruction
// ): decoded is DecodedInitializeMint2Instruction {
//     return decoded.data.instruction === TokenInstruction.InitializeMint2;
// }
