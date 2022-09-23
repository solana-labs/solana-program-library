import { CompiledInnerInstruction, MessageCompiledInstruction, } from '@solana/web3.js';
import { BlockTransaction } from '../../actions/fetch';
import { Sqlite3 } from '../../db/sqlite3';
import { ingestInstruction } from '../instruction';

export type ZippedInstruction = {
    instructionIndex: number,
    instruction: MessageCompiledInstruction,
    innerInstructions: CompiledInnerInstruction[],
}

/// Similar to `order_instructions` in `/nft_ingester/src/utils/instructions.rs`
function zipInstructions(
    instructions: MessageCompiledInstruction[],
    innerInstructions: CompiledInnerInstruction[],
): ZippedInstruction[] {
    // Map of which instructions have corresponding innerInstructions
    const innerIxMap: Map<number, CompiledInnerInstruction> = new Map();
    for (const innerIx of innerInstructions) {
        innerIxMap.set(innerIx.index, innerIx);
    }

    // Zip outer with (flattened) inner instructions
    const zippedIxs: ZippedInstruction[] = [];
    for (const [instructionIndex, instruction] of instructions.entries()) {
        zippedIxs.push({
            instructionIndex,
            instruction,
            innerInstructions: innerIxMap.has(instructionIndex) ? [innerIxMap.get(instructionIndex)!] : []
        })
    }
    return zippedIxs;
}

/**
 * Ingests transactions
 */
export async function ingestTransaction(
    db: Sqlite3,
    slot: number,
    transaction: BlockTransaction,
) {
    const instructions = transaction.transaction.message.compiledInstructions;
    const innerInstructions: CompiledInnerInstruction[] = transaction.meta?.innerInstructions ?? [];

    const zippedInstructions = zipInstructions(instructions, innerInstructions);
    for (const zippedInstruction of zippedInstructions) {
        await ingestInstruction(db, slot, transaction, zippedInstruction);
    }
}