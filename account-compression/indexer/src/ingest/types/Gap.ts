/**
 * Only need previousSlot versus currentSlot
 */
export type GapInfo = {
    previousSeq: number,
    previousSlot: number,
    previousTransactionId: string,
    currentSeq: number,
    currentSlot: number,
    currentTransactionId: string,
}