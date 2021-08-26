import BN from "bn.js";

// --------------------------------------- game state

export class GameState {
    TYPE = 0;
    round_id = new BN(0);
    round_init_time = new BN(0);
    round_inc_time_per_key = new BN(0);
    round_max_time = new BN(0);
    version = new BN(0);
    mint = new Uint8Array(32);
    game_creator = new Uint8Array(32);
    community_wallet = new Uint8Array(32);
    p3d_wallet = new Uint8Array(32);

    constructor(fields?: Partial<GameState>) {
        Object.assign(this, fields);
    }
}

export const gameSchema = new Map([[GameState, {
    kind: 'struct',
    fields: [
        ['TYPE', 'u8'],
        ['round_id', 'u64'],
        ['round_init_time', 'u64'], //borsh doesn't understand i64 - only u64
        ['round_inc_time_per_key', 'u64'],
        ['round_max_time', 'u64'],
        ['version', 'u64'],
        ['mint', [32]],
        ['game_creator', [32]],
        ['community_wallet', [32]],
        ['p3d_wallet', [32]],
    ]
}]])

// --------------------------------------- round state

export class SolByTeam {
    whale = new BN(0);
    bear = new BN(0);
    snek = new BN(0);
    bull = new BN(0);

    constructor(fields?: Partial<SolByTeam>) {
        Object.assign(this, fields);
    }
}

export const solByTeamSchema = new Map([[SolByTeam, {
    kind: 'struct',
    fields: [
        ['whale', 'u128'],
        ['bear', 'u128'],
        ['snek', 'u128'],
        ['bull', 'u128'],
    ]
}]])

export class RoundState {
    TYPE = 0;
    round_id = new BN(0);
    lead_player_pk = new Array(32);
    lead_player_team = 0;
    start_time = new BN(0);
    end_time = new BN(0);
    ended = 0;
    accum_keys = new BN(0);
    accum_sol_pot = new BN(0);
    accum_sol_by_team = new SolByTeam();
    //shares
    accum_community_share = new BN(0);
    accum_airdrop_share = new BN(0);
    accum_next_round_share = new BN(0);
    accum_aff_share = new BN(0);
    accum_p3d_share = new BN(0);
    accum_f3d_share = new BN(0);
    still_in_play = new BN(0);
    final_prize_share = new BN(0);
    //withdrawals
    withdrawn_com = new BN(0);
    withdrawn_next_round = new BN(0);
    withdrawn_p3d = new BN(0);
    //airdrop
    airdrop_tracker = new BN(0);

    constructor(fields?: RoundState) {
        Object.assign(this, fields);
    }
}

export const roundSchema = new Map([[RoundState, {
    kind: 'struct',
    fields: [
        ['TYPE', 'u8'],
        ['round_id', 'u64'],
        ['lead_player_pk', [32]],
        ['lead_player_team', 'u8'],
        ['start_time', 'u64'],
        ['end_time', 'u64'],
        ['ended', 'u8'],
        ['accum_keys', 'u128'],
        ['accum_sol_pot', 'u128'],
        ['accum_sol_by_team', [64]],
        ['accum_community_share', 'u128'],
        ['accum_airdrop_share', 'u128'],
        ['accum_next_round_share', 'u128'],
        ['accum_aff_share', 'u128'],
        ['accum_p3d_share', 'u128'],
        ['accum_f3d_share', 'u128'],
        ['still_in_play', 'u128'],
        ['final_prize_share', 'u128'],
        ['withdrawn_com', 'u128'],
        ['withdrawn_next_round', 'u128'],
        ['withdrawn_p3d', 'u128'],
        ['airdrop_tracker', 'u64'],
    ]
}]])

// --------------------------------------- player state

export class PlayerRoundState {
    TYPE = 0;
    player_pk = new Uint8Array(32);
    round_id = new BN(0);
    last_affiliate_pk = new Uint8Array(32);
    accum_keys = new BN(0);
    accum_sol_added = new BN(0);
    accum_winnings = new BN(0);
    accum_aff = new BN(0);
    withdrawn_winnings = new BN(0);
    withdrawn_aff = new BN(0);
    withdrawn_f3d = new BN(0);

    constructor(fields?: Partial<PlayerRoundState>) {
        Object.assign(this, fields);
    }
}

export const playerRoundStateSchema = new Map([[PlayerRoundState, {
    kind: 'struct',
    fields: [
        ['TYPE', 'u8'],
        ['player_pk', [32]],
        ['round_id', 'u64'],
        ['last_affiliate_pk', [32]],
        ['accum_keys', 'u128'],
        ['accum_sol_added', 'u128'],
        ['accum_winnings', 'u128'],
        ['accum_aff', 'u128'],
        ['withdrawn_winnings', 'u128'],
        ['withdrawn_aff', 'u128'],
        ['withdrawn_f3d', 'u128'],
    ]
}]])