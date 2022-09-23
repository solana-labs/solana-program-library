import {
    PublicKey,
    Connection
} from '@solana/web3.js';
import { GapInfo } from '../../ingest/types/Gap';
import { getAllSlots } from './getAllSlots';

export async function getAllSlotsMissing(
    connection: Connection,
    treeId: PublicKey,
    gaps: GapInfo[],
): Promise<number[]> {
    let slots: number[] = []
    for (const gap of gaps) {
        slots = slots.concat(await getAllSlots(connection, treeId, gap.previousTransactionId, gap.currentTransactionId))
    }
    return slots;
}