import {
    deserializeAccountCompressionEvent,
    ChangeLogEventV1,
    SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
    SPL_NOOP_PROGRAM_ID
} from '@solana/spl-account-compression';
import {
    BorshInstructionCoder,
    Idl,
} from '@project-serum/anchor';
import SplAccountCompressionIdl from '@solana/spl-account-compression/idl/spl_account_compression.json';
import {
    PublicKey
} from '@solana/web3.js';
import { BlockTransaction, getTransactionSignature } from '../../actions/fetch';
import { Sqlite3 } from '../../db/sqlite3';
import { ZippedInstruction } from '../transaction';
import * as bs58 from 'bs58';

function getInstructionProgramId(ix: ZippedInstruction, tx: BlockTransaction): PublicKey {
    const accountKeys = tx.transaction.message.getAccountKeys();
    return accountKeys.get(ix.instruction.programIdIndex)!
}

export type SplAccountCompressionInstructionName = 'initEmptyMerkleTree'
    | 'append'
    | 'replaceLeaf'
    | 'verifyLeaf'
    | 'transferAuthority'
    | 'insertOrAppend';


function getChangelogs(zippedInstruction: ZippedInstruction, transaction: BlockTransaction): ChangeLogEventV1 {
    const innerInstructionPkg = zippedInstruction.innerInstructions;
    if (innerInstructionPkg.length != 1) {
        throw Error(`Incorrect number of inner instructions zipped: ${innerInstructionPkg.length}`)
    }
    const innerInstructions = innerInstructionPkg[0].instructions
    if (innerInstructions.length != 1) {
        throw Error(`Incorrect number of CPIs made by SPL Account Compression: ${innerInstructions.length}`)
    }

    const noopInstruction = innerInstructions[0];
    const accountKeys = transaction.transaction.message.getAccountKeys();
    const programId = accountKeys.get(noopInstruction.programIdIndex);
    if (!programId.equals(SPL_NOOP_PROGRAM_ID)) {
        throw Error(`SPL Account Compression made CPI to unknown program: ${programId.toBase58()}`)
    }

    return deserializeAccountCompressionEvent(bs58.decode(noopInstruction.data));
}

/**
 * Ingests relevant transaction to a database
 */
export async function ingestInstruction(
    db: Sqlite3,
    slot: number,
    transaction: BlockTransaction,
    zippedInstruction: ZippedInstruction
) {
    const programId = getInstructionProgramId(zippedInstruction, transaction);

    if (programId.equals(SPL_ACCOUNT_COMPRESSION_PROGRAM_ID)) {
        const coder = new BorshInstructionCoder(SplAccountCompressionIdl as Idl);
        const decodedIx = coder.decode(Buffer.from(zippedInstruction.instruction.data));
        switch (decodedIx.name as SplAccountCompressionInstructionName) {
            // These all have 1 change log
            case 'initEmptyMerkleTree':
            case 'append':
            case 'replaceLeaf':
            case 'insertOrAppend':
                const changeLog = getChangelogs(zippedInstruction, transaction);
                console.log(changeLog);
                await db.updateChangeLogs(
                    changeLog,
                    getTransactionSignature(transaction),
                    slot,
                );
                break;
            // These have no change logs
            case 'transferAuthority':
            case 'verifyLeaf':
                break;
        }
    } else if (zippedInstruction.innerInstructions.length) {
        // Todo(ngundotra):
        // Necessary for composing with a program that uses compression
    }
}