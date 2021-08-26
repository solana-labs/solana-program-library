import {
    aliceKp,
    bobKp, changeGlobalPlayerState,
    endRound,
    getPlayerRoundState,
    getTokenAccBalance,
    initGame,
    initRound,
    prepareTestEnv,
    purchaseKeys,
    withdrawSol,
    wSolAliceAcc,
    wSolBobAcc,
    wSolPot
} from "../src/main";
import BN from "bn.js";
import {assert, waitForRoundtoEnd} from "./utils";
import {LAMPORTS_PER_SOL, PublicKey} from "@solana/web3.js";
import {verifyPlayerState, verifyRoundState} from "./purchaseKeys.test";

describe('withdraw sol', () => {
    it('withdraw f3d (before round end)', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await purchaseKeys(aliceKp, wSolAliceAcc, 1);
        await verifyPotAndAccBalances(wSolAliceAcc, 0, 1); //before

        //43% out of the 1 sol = 0.43 sol
        //at this point the player only accumulated f3d
        let solMoved = 0.43;

        await withdrawSol(aliceKp, wSolAliceAcc);
        await verifyPotAndAccBalances(wSolAliceAcc, solMoved, 1); //after
        await withdrawSol(aliceKp, wSolAliceAcc);
        await verifyPotAndAccBalances(wSolAliceAcc, solMoved, 1); //verify 2nd attempt does not move more out
        await verifyPlayerWithdrawals(0, 0, solMoved);
    })
})

describe('withdraw sol', () => {
    it('withdraw grand prize (after round end, single participant)', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await purchaseKeys(aliceKp, wSolAliceAcc, 1);
        await waitForRoundtoEnd();
        await endRound();
        await verifyPotAndAccBalances(wSolAliceAcc, 0, 1); //before

        //0.43 from deposit + another 25% from 0.43 that went to pot
        let f3dProceeds = 0.43 + 0.25 * 0.43;
        //48% from 0.43 that went to pot
        let grandPrize = 0.48 * 0.43;
        let solMoved = f3dProceeds + grandPrize;

        await withdrawSol(aliceKp, wSolAliceAcc);
        await verifyPotAndAccBalances(wSolAliceAcc, solMoved, 1); //after
        await withdrawSol(aliceKp, wSolAliceAcc);
        await verifyPotAndAccBalances(wSolAliceAcc, solMoved, 1); //verify 2nd attempt does not move more out
        await verifyPlayerWithdrawals(grandPrize, 0, f3dProceeds);
    })
})

describe('withdraw sol', () => {
    it('withdraw grand prize (after round end, multi participant)', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        await purchaseKeys(bobKp, wSolBobAcc, 1);
        await purchaseKeys(aliceKp, wSolAliceAcc, 1);
        await waitForRoundtoEnd();
        await endRound();
        await verifyPotAndAccBalances(wSolAliceAcc, 0, 2); //before

        //below calc for Alice, who is the winner
        //there are two equal deposits now, but she only gets ~half of them
        let f3dProceeds = (0.43 * 2 + 0.25 * 0.43 * 2) * 12811 / 25964;
        //grand prize now 2x big
        let grandPrize = 0.48 * 0.43 * 2;
        //need to do some rounding to match the program
        let solMoved = Math.floor((f3dProceeds + grandPrize) * LAMPORTS_PER_SOL) / LAMPORTS_PER_SOL;

        await withdrawSol(aliceKp, wSolAliceAcc);
        await verifyPotAndAccBalances(wSolAliceAcc, solMoved, 2); //after
        await withdrawSol(aliceKp, wSolAliceAcc);
        await verifyPotAndAccBalances(wSolAliceAcc, solMoved, 2); //verify 2nd attempt does not move more out
        await verifyPlayerWithdrawals(grandPrize, 0, f3dProceeds);
    })
})

describe('withdraw sol', () => {
    it('completes a purchases with an affiliate / withdrawal for affiliate', async () => {
        // --------------------------------------- purchase with an affiliate
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        //alice buys keys, this time adding bob as an affiliate
        await purchaseKeys(aliceKp, wSolAliceAcc, 1, bobKp.publicKey);
        await verifyRoundState(1, aliceKp.publicKey, 13153, true);
        await verifyPlayerState(aliceKp.publicKey, 13153);
        //aditional checks
        let aliceState = await getPlayerRoundState();
        assert(new PublicKey(aliceState.last_affiliate_pk).toString() == bobKp.publicKey.toString());
        //this is to set player state to bob, so we can check his affiliate earnings
        await purchaseKeys(bobKp, wSolBobAcc, 1);
        let bobState = await getPlayerRoundState();
        assert(bobState.accum_aff.eq(new BN(LAMPORTS_PER_SOL / 10)));

        // --------------------------------------- affiliate withdrawal
        await verifyPotAndAccBalances(wSolBobAcc, 0, 2); //before
        let f3dProceeds = (0.43 * 2) * 12811 / 25964;
        let affProceeds = 0.1;
        let solMoved = Math.floor((f3dProceeds + affProceeds) * LAMPORTS_PER_SOL) / LAMPORTS_PER_SOL;
        await withdrawSol(bobKp, wSolBobAcc);
        await verifyPotAndAccBalances(wSolBobAcc, solMoved, 2, 'ceil'); //after
        await withdrawSol(bobKp, wSolBobAcc);
        await verifyPotAndAccBalances(wSolBobAcc, solMoved, 2, 'ceil'); //verify 2nd attempt does not move more out
        await verifyPlayerWithdrawals(0, affProceeds, f3dProceeds);
    })
})

describe('withdraw sol', () => {
    it('fails to withdraw if the player hasnt deposited anything', async () => {
        await prepareTestEnv();
        await initGame();
        await initRound(1);
        //alice deposits - so there is money in the game
        await purchaseKeys(aliceKp, wSolAliceAcc, 1);
        //bob tries to withdraw without having deposited anything
        await changeGlobalPlayerState(bobKp);
        await expect(withdrawSol(bobKp, wSolBobAcc)).rejects.toThrow("custom program error: 0x8");
        await verifyPotAndAccBalances(wSolBobAcc, 0, 1, 'floor', 100);
    })
})

async function verifyPotAndAccBalances(
    playerAcc: PublicKey,
    solMoved: number,
    multiple: number,
    roundDir: string = 'floor',
    startingBalance: number = 99,
) {
    let potBalance = await getTokenAccBalance(wSolPot);
    let playerAccBalance = await getTokenAccBalance(playerAcc);
    if (roundDir == 'floor') {
        assert(new BN(potBalance.amount).eq(new BN(Math.floor((multiple - solMoved) * LAMPORTS_PER_SOL))));
    } else {
        assert(new BN(potBalance.amount).eq(new BN(Math.ceil((multiple - solMoved) * LAMPORTS_PER_SOL))));
    }
    assert(new BN(playerAccBalance.amount).eq(new BN((startingBalance + solMoved) * LAMPORTS_PER_SOL)));
}

async function verifyPlayerWithdrawals(
    winWithdrawal: number,
    affWithdrawal: number,
    f3dWithdrawal: number,
) {
    let playerState = await getPlayerRoundState();
    assert(playerState.withdrawn_winnings.eq(new BN(winWithdrawal * LAMPORTS_PER_SOL)))
    assert(playerState.withdrawn_aff.eq(new BN(affWithdrawal * LAMPORTS_PER_SOL)));
    assert(playerState.withdrawn_f3d.eq(new BN(f3dWithdrawal * LAMPORTS_PER_SOL)))
}