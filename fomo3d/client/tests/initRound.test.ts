import {
    aliceKp,
    endRound,
    getGameState,
    getRoundState,
    getTokenAccBalance,
    initGame,
    initRound,
    prepareTestEnv,
    purchaseKeys,
    ROUND_INIT_TIME,
    wSolAliceAcc,
    wSolPot
} from "../src/main";
import BN from "bn.js";
import {assert, waitForRoundtoEnd} from "./utils";

describe('init round', () => {
    it('successfully inits a new round', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        let gameState = await getGameState();
        let roundState = await getRoundState();
        assert(gameState.round_id.toString() == "1");

        //NOTE: this will fail if your local validator node is out of sync
        //do solana-test-validator --reset then, and follow the rest of instructions in README.md
        let now = new BN(Date.now() / 1000);
        let now_minus_1min = now.sub(new BN(60));
        let end_time = now.add(new BN(ROUND_INIT_TIME));
        let end_time_minus_1min = end_time.sub(new BN(60));
        assert(roundState.start_time.gte(now_minus_1min));
        assert(roundState.start_time.lte(now));
        assert(roundState.end_time.gte(end_time_minus_1min));
        assert(roundState.end_time.lte(end_time));

        let must_be_zero: BN[] = [
            roundState.accum_keys,
            roundState.accum_sol_pot,
            roundState.accum_community_share,
            roundState.accum_airdrop_share,
            roundState.accum_next_round_share,
            roundState.accum_aff_share,
            roundState.accum_p3d_share,
            roundState.accum_f3d_share,
            roundState.still_in_play,
            roundState.final_prize_share,
            roundState.withdrawn_com,
            roundState.withdrawn_next_round,
            roundState.withdrawn_p3d,
            roundState.airdrop_tracker,
        ]
        must_be_zero.forEach(num => assert(num.isZero()))
    })
})

describe('init round', () => {
    it('refuses to init the same round twice', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        //in theory expecting an AlreadyExists error (0x9),
        // but because we're now passing an intialized PDA, the ownership check (0x8) fails first
        await expect(initRound(1)).rejects.toThrow("custom program error: 0x8");
    })
})

describe('init round', () => {
    it('inits a 2nd round after previous successfully closed', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await purchaseKeys(aliceKp, wSolAliceAcc,1); //needed to initialize playerRoundState
        await waitForRoundtoEnd(); //needed to be able to close the round
        await endRound(); //closing the round
        let roundState = await getRoundState();
        await initRound(2);
        //verify that funds correctly migrated from 1st round's pot to 2nd round
        let round2State = await getRoundState();
        let round2Pot = await getTokenAccBalance(wSolPot);
        assert(roundState.accum_next_round_share.eq(round2State.accum_sol_pot));
        assert(round2State.accum_sol_pot.eq(new BN(round2Pot.amount)));
        //check withdrawl recorded correctly in old round's state
        let roundStateUpdated = await getRoundState();
        assert(roundStateUpdated.withdrawn_next_round.eq(roundStateUpdated.accum_next_round_share));
    })
})

describe('init round', () => {
    it('refuses to init a 2nd round before previous closed', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await expect(initRound(2)).rejects.toThrow("custom program error: 0xd");
    })
})