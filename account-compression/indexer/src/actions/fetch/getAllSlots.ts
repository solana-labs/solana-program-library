import { Connection, PublicKey } from '@solana/web3.js';

function onlyUnique(value: any, index: number, self: any) {
    return self.indexOf(value) === index;
}

/**
 * @returns all the slots for a given address
 */
export async function getAllSlots(
    connection: Connection,
    treeId: PublicKey,
    afterSig?: string,
    untilSig?: string,
): Promise<number[]> {
    // todo: paginate
    let lastAddress: string | null = untilSig ?? null;
    let done = false;
    const history: number[] = [];

    const baseOpts = afterSig ? { until: afterSig } : {};
    while (!done) {
        let opts = lastAddress ? { before: lastAddress } : {};
        const finalOpts = { ...baseOpts, ...opts };
        const rawSigs = (await connection.getSignaturesForAddress(treeId, finalOpts))
        if (rawSigs.length === 0) {
            return [];
        } else if (rawSigs.length < 1000) {
            done = true;
        }
        const sigs = rawSigs.filter((confirmedSig) => !confirmedSig.err);
        lastAddress = sigs[sigs.length - 1].signature;
        sigs.map((sigInfo) => {
            history.push(sigInfo.slot);
        })
    }

    return history.reverse().filter(onlyUnique);
}