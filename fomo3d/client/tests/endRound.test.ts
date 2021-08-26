import {
    aliceKp,
    bobKp,
    endRound,
    getRoundState,
    initGame,
    initRound,
    prepareTestEnv,
    purchaseKeys,
    wSolAliceAcc,
    wSolBobAcc
} from "../src/main";
import {assert, waitForRoundtoEnd} from "./utils";
import BN from "bn.js";
import {LAMPORTS_PER_SOL} from "@solana/web3.js";

describe('end round', () => {
    it('ends round when time elapses', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await purchaseKeys(aliceKp, wSolAliceAcc, 1);
        await purchaseKeys(bobKp, wSolBobAcc, 1);
        await waitForRoundtoEnd();
        let openRoundState = await getRoundState();
        await endRound();
        let closedRoundState = await getRoundState();

        //team bear
        let toBeDivided = 0.43 * 2 * LAMPORTS_PER_SOL;
        let commShare = 0.02 * toBeDivided;
        let grandPrize = 0.48 * toBeDivided;
        let f3dShare = 0.25 * toBeDivided;
        let p3dShare = 0 * toBeDivided;
        let nextRoundShare = 0.25 * toBeDivided;

        assert(closedRoundState.accum_community_share.eq(
            new BN(commShare).add(openRoundState.accum_community_share)
        ));
        assert(closedRoundState.final_prize_share.eq(
            new BN(grandPrize).add(openRoundState.final_prize_share)
        ));
        assert(closedRoundState.accum_f3d_share.eq(
            new BN(f3dShare).add(openRoundState.accum_f3d_share)
        ));
        assert(closedRoundState.accum_p3d_share.eq(
            new BN(p3dShare).add(openRoundState.accum_p3d_share)
        ));
        assert(closedRoundState.accum_next_round_share.eq(
            new BN(nextRoundShare).add(openRoundState.accum_next_round_share)
        ));
        assert(closedRoundState.still_in_play.eq(new BN (0)));
        assert(openRoundState.ended == 0);
        assert(closedRoundState.ended == 1);

        //player accumulated earnings are checked during player withdrawal
    })
})

describe('end round', () => {
    it('refuses to end round too early', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await purchaseKeys(aliceKp, wSolAliceAcc, 1);
        await expect(endRound()).rejects.toThrow("custom program error: 0xd");
    })
})