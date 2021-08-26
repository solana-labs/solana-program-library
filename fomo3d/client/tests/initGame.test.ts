import {
    gameCreatorKp,
    getGameState,
    initGame,
    prepareTestEnv,
    ROUND_INC_TIME_PER_KEY,
    ROUND_INIT_TIME,
    ROUND_MAX_TIME,
    wSolComAcc,
    wSolMint,
    wSolP3dAcc
} from "../src/main";
import BN from "bn.js";
import {assert} from "./utils";
import {PublicKey} from "@solana/web3.js";

describe('init game', () => {
    it('successfully inits a new game', async () => {
        await prepareTestEnv();
        await initGame();
        let gameState = await getGameState();
        assert(gameState.round_id.isZero());
        assert(gameState.round_init_time.eq(new BN(ROUND_INIT_TIME)));
        assert(gameState.round_inc_time_per_key.eq(new BN(ROUND_INC_TIME_PER_KEY)));
        assert(gameState.round_max_time.eq(new BN(ROUND_MAX_TIME)));
        assert(new PublicKey(gameState.mint).toString() == wSolMint.publicKey.toString());
        assert(new PublicKey(gameState.game_creator).toString() == gameCreatorKp.publicKey.toString());
        assert(new PublicKey(gameState.community_wallet).toString() == wSolComAcc.toString());
        assert(new PublicKey(gameState.p3d_wallet).toString() == wSolP3dAcc.toString());
    })
})

describe('init game', () => {
    it('fails to init twice', async () => {
        await prepareTestEnv();
        await initGame();
        //in theory expecting an AlreadyExists error (0x9),
        // but because we're now passing an intialized PDA, the ownership check (0x8) fails first
        await expect(initGame()).rejects.toThrow("custom program error: 0x8");
    })
})