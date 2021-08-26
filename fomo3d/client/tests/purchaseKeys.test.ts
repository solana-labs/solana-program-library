import {
    aliceKp,
    bobKp,
    getPlayerRoundState,
    getRoundState,
    getTokenAccBalance,
    initGame,
    initRound,
    prepareTestEnv,
    purchaseKeys,
    ROUND_INIT_TIME,
    wSolAliceAcc,
    wSolBobAcc,
    wSolPot
} from "../src/main";
import BN from "bn.js";
import {assert, waitForRoundtoEnd} from "./utils";
import {LAMPORTS_PER_SOL, PublicKey} from "@solana/web3.js";

describe('purchase keys', () => {
    it('completes & records a number of key purchases', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        //alice buys keys
        await purchaseKeys(aliceKp, wSolAliceAcc, 1);
        await verifyRoundState(1, aliceKp.publicKey, 13153);
        await verifyPlayerState(aliceKp.publicKey, 13153);
        await verifyPotState(1);
        //bob buys keys
        await purchaseKeys(bobKp, wSolBobAcc, 1);
        await verifyRoundState(2, bobKp.publicKey, 25964);
        //2nd player gets fewer keys since they're later to the game
        await verifyPlayerState(bobKp.publicKey, 12811);
        await verifyPotState(2);
    })
})

describe('purchase keys', () => {
    it('refuses purchase if round has ended', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await waitForRoundtoEnd();
        await expect(
            purchaseKeys(aliceKp, wSolAliceAcc, 1, bobKp.publicKey)
        ).rejects.toThrow("custom program error: 0xe");
    })
})

describe('purchase keys', () => {
    it('limits purchase to 1 sol until 100 sol total accumulated', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await purchaseKeys(aliceKp, wSolAliceAcc, 1, bobKp.publicKey);
        await expect(
            purchaseKeys(aliceKp, wSolAliceAcc, 1, bobKp.publicKey)
        ).rejects.toThrow("custom program error: 0x3");
    })
})

describe('purchase keys', () => {
    it('requires a minimum purchase of at least 1 key', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await expect(
            purchaseKeys(aliceKp, wSolAliceAcc, 1 / LAMPORTS_PER_SOL, bobKp.publicKey)
        ).rejects.toThrow("custom program error: 0x3");
    })
})

describe('purchase keys', () => {
    it('increments time correctly (< max)', async () => {
        await prepareTestEnv();
        await initGame(ROUND_INIT_TIME, 2);
        await initRound(1);
        let prePurchaseState = await getRoundState();
        await purchaseKeys(aliceKp, wSolAliceAcc, 1, bobKp.publicKey);
        let postPurchaseState = await getRoundState();
        let timeDiff = postPurchaseState.end_time.sub(prePurchaseState.end_time).toNumber();
        assert(24000 < timeDiff && timeDiff < 28000);
    })
})

describe('purchase keys', () => {
    it('increments time correctly (> max)', async () => {
        await prepareTestEnv();
        await initGame(ROUND_INIT_TIME, 2, 10000);
        await initRound(1);
        let prePurchaseState = await getRoundState();
        await purchaseKeys(aliceKp, wSolAliceAcc, 1, bobKp.publicKey);
        let postPurchaseState = await getRoundState();
        let timeDiff = postPurchaseState.end_time.sub(prePurchaseState.end_time).toNumber();
        assert(9500 < timeDiff && timeDiff < 10500);
    })
})

export async function verifyRoundState(multiple: number, leadPlayer: PublicKey, keys: number, affAdded = false) {
    let roundState = await getRoundState();
    assert(new PublicKey(roundState.lead_player_pk).toString() == leadPlayer.toString());
    assert(roundState.lead_player_team == 1); //team bear
    assert(roundState.accum_keys.eq(new BN(keys)));
    assert(roundState.accum_sol_pot.eq(new BN(LAMPORTS_PER_SOL * multiple)));
    assert(roundState.accum_sol_by_team.bear.eq(new BN(LAMPORTS_PER_SOL * multiple)));
    assert(roundState.accum_community_share.eq(new BN(LAMPORTS_PER_SOL / 50 * multiple)));
    assert(roundState.accum_airdrop_share.eq(new BN(LAMPORTS_PER_SOL / 100 * multiple)));
    assert(roundState.accum_next_round_share.eq(new BN(LAMPORTS_PER_SOL / 100 * multiple)));
    if (affAdded) {
        assert(roundState.accum_aff_share.eq(new BN(LAMPORTS_PER_SOL / 10 * multiple)));
        assert(roundState.accum_p3d_share.eq(new BN(0)));
    } else {
        assert(roundState.accum_aff_share.eq(new BN(0)));
        assert(roundState.accum_p3d_share.eq(new BN(LAMPORTS_PER_SOL / 10 * multiple)));
    }
    assert(roundState.accum_f3d_share.eq(new BN(43 * LAMPORTS_PER_SOL / 100 * multiple)));
    assert(roundState.still_in_play.eq(new BN(43 * LAMPORTS_PER_SOL / 100 * multiple)));
    assert(roundState.airdrop_tracker.eq(new BN(multiple)));
}

export async function verifyPlayerState(player: PublicKey, keys: number) {
    let playerState = await getPlayerRoundState();
    assert(new PublicKey(playerState.player_pk).toString() == player.toString());
    assert(playerState.round_id.eq(new BN(1)));
    assert(playerState.accum_keys.eq(new BN(keys)));
    assert(playerState.accum_sol_added.eq(new BN(LAMPORTS_PER_SOL)));
}

async function verifyPotState(multiple: number) {
    let potState = await getTokenAccBalance(wSolPot);
    assert(new BN(potState.amount).eq(new BN(LAMPORTS_PER_SOL * multiple)));
}

//todo no good way to test the airdrop unforunately
// what's more - it can occasionally mess up the tests above (0.1% chance)