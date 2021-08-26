import {
    aliceKp,
    bobKp,
    endRound,
    getRoundState,
    getTokenAccBalance,
    initGame,
    initRound,
    prepareTestEnv,
    purchaseKeys,
    withdrawCom,
    wSolAliceAcc,
    wSolBobAcc,
    wSolComAcc,
    wSolPot
} from "../src/main";
import BN from "bn.js";
import {assert, waitForRoundtoEnd} from "./utils";
import {LAMPORTS_PER_SOL} from "@solana/web3.js";

describe('withdraw community share', () => {
    it('works before round ends', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await purchaseKeys(aliceKp, wSolAliceAcc, 1);
        await purchaseKeys(bobKp, wSolBobAcc, 1);
        await verifyPotAndComAccBalances(0, 2); //before

        //community share = always 2%.
        let solMovedPerPlayer = 0.02;

        await withdrawCom();
        await verifyPotAndComAccBalances(solMovedPerPlayer, 2); //after
        await withdrawCom();
        await verifyPotAndComAccBalances(solMovedPerPlayer, 2); //verify 2nd attempt does not move more out
    })
})

describe('withdraw community share', () => {
    it('works after round ends', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await purchaseKeys(aliceKp, wSolAliceAcc, 1);
        await purchaseKeys(bobKp, wSolBobAcc, 1);
        await verifyPotAndComAccBalances(0, 2); //before
        await waitForRoundtoEnd();
        await endRound();

        //2% from purchase + 2% from the final pot.
        let solMovedPerPlayer = 0.02 + 0.43 * 0.02;

        await withdrawCom();
        await verifyPotAndComAccBalances(solMovedPerPlayer, 2); //after
        await withdrawCom();
        await verifyPotAndComAccBalances(solMovedPerPlayer, 2); //verify 2nd attempt does not move more out
        await verifyComWithdrawals(solMovedPerPlayer * 2);
    })
})

async function verifyPotAndComAccBalances(
    solMoved: number,
    multiple: number,
) {
    let potBalance = await getTokenAccBalance(wSolPot);
    let comAccBalance = await getTokenAccBalance(wSolComAcc);
    assert(new BN(potBalance.amount).eq(new BN(Math.floor((multiple - solMoved * multiple) * LAMPORTS_PER_SOL))));
    assert(new BN(comAccBalance.amount).eq(new BN((multiple * solMoved) * LAMPORTS_PER_SOL)));
}

async function verifyComWithdrawals(
    amount: number,
) {
    let roundState = await getRoundState();
    assert(roundState.accum_community_share.eq(new BN(amount * LAMPORTS_PER_SOL)));
    assert(roundState.withdrawn_com.eq(new BN(amount * LAMPORTS_PER_SOL)));
}

