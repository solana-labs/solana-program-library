import {Token, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import {
    Connection,
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
    Transaction,
    TransactionInstruction
} from "@solana/web3.js";
import BN from "bn.js";
import * as borsh from 'borsh';
import {
    gameSchema,
    GameState,
    PlayerRoundState,
    playerRoundStateSchema,
    roundSchema,
    RoundState,
    SolByTeam,
    solByTeamSchema,
} from "./layout";

// ============================================================================= globals & consts
export let connection: Connection;
export const FOMO_PROG_ID = new PublicKey("2HEMUe2d8HFfCMoBARcP5HSoKB5RRSg8dvLG4TVh2fHB");

export const gameCreatorKp: Keypair = Keypair.fromSecretKey(Uint8Array.from([208, 175, 150, 242, 88, 34, 108, 88, 177, 16, 168, 75, 115, 181, 199, 242, 120, 4, 78, 75, 19, 227, 13, 215, 184, 108, 226, 53, 111, 149, 179, 84, 137, 121, 79, 1, 160, 223, 124, 241, 202, 203, 220, 237, 50, 242, 57, 158, 226, 207, 203, 188, 43, 28, 70, 110, 214, 234, 251, 15, 249, 157, 62, 80]));
export const aliceKp: Keypair = Keypair.fromSecretKey(Uint8Array.from([201, 101, 147, 128, 138, 189, 70, 190, 202, 49, 28, 26, 32, 21, 104, 185, 191, 41, 20, 171, 3, 144, 4, 26, 169, 73, 180, 171, 71, 22, 48, 135, 231, 91, 179, 215, 3, 117, 187, 183, 96, 74, 154, 155, 197, 243, 114, 104, 20, 123, 105, 47, 181, 123, 171, 133, 73, 181, 102, 41, 236, 78, 210, 176]));
export const bobKp: Keypair = Keypair.fromSecretKey(Uint8Array.from([177, 217, 193, 155, 63, 150, 164, 184, 81, 82, 121, 165, 202, 87, 86, 237, 218, 226, 212, 201, 167, 170, 149, 183, 59, 43, 155, 112, 189, 239, 231, 110, 162, 218, 184, 20, 108, 2, 92, 114, 203, 184, 223, 69, 137, 206, 102, 71, 162, 0, 127, 63, 170, 96, 137, 108, 228, 31, 181, 113, 57, 189, 30, 76]));

export let gameState: PublicKey;
export let roundState: PublicKey;
export let playerState: PublicKey;

export let wSolMint: Token;
export let wSolAliceAcc: PublicKey;
export let wSolBobAcc: PublicKey;
export let wSolComAcc: PublicKey;
export let wSolP3dAcc: PublicKey;
export let wSolPot: PublicKey;

export let version: number;
export let round = 1;

//setting these low so that we can run tests
//actual recommended numbers are 1h / 30s / 24h
export const ROUND_INIT_TIME = 2;
export const ROUND_INC_TIME_PER_KEY = 0;
export const ROUND_MAX_TIME = 24 * 60 * 60;

// ============================================================================= helpers

async function getConnection() {
    const url = 'http://localhost:8899';
    connection = new Connection(url, 'processed');
    const version = await connection.getVersion();
    console.log('connection to cluster established:', url, version);
}

async function prepareAndSendTx(instructions: TransactionInstruction[], signers: Signer[]) {
    const tx = new Transaction().add(...instructions);
    const sig = await sendAndConfirmTransaction(connection, tx, signers);
    console.log(sig);
}

async function generateCreateAccIx(newAccountPubkey: PublicKey, space: number): Promise<TransactionInstruction> {
    return SystemProgram.createAccount({
        programId: FOMO_PROG_ID,
        fromPubkey: gameCreatorKp.publicKey,
        newAccountPubkey,
        space,
        lamports: await connection.getMinimumBalanceForRentExemption(space),
    });
}

async function createMintAccount(): Promise<Token> {
    return Token.createMint(
        connection,
        gameCreatorKp,
        gameCreatorKp.publicKey,
        null,
        0,
        TOKEN_PROGRAM_ID,
    );
}

async function createAndFundTokenAccount(mint: Token, owner: PublicKey, mintAmount: number = 0): Promise<PublicKey> {
    const tokenUserPk = await mint.createAccount(owner);
    if (mintAmount > 0) {
        await mint.mintTo(tokenUserPk, gameCreatorKp.publicKey, [], mintAmount);
    }
    return tokenUserPk;
}

export async function changeGlobalPlayerState(player: Keypair) {
    let bump;
    [playerState, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(`pr${player.publicKey.toBase58().substring(0, 12)}${round}${version}`)],
        FOMO_PROG_ID,
    )
    console.log('global player state now:', playerState.toBase58());
}

// ============================================================================= state getters

export async function getGameState() {
    let gameStateInfo = await connection.getAccountInfo(gameState);
    let gameStateData = borsh.deserialize(gameSchema, GameState, gameStateInfo?.data as Buffer);
    console.log(gameStateData);
    return gameStateData
}

export async function getRoundState() {
    let roundStateInfo = await connection.getAccountInfo(roundState);
    let roundStateData = borsh.deserialize(roundSchema, RoundState, roundStateInfo?.data as Buffer);
    let solByTeamData = borsh.deserialize(solByTeamSchema, SolByTeam, roundStateData.accum_sol_by_team as any as Buffer);
    roundStateData.accum_sol_by_team = solByTeamData;
    console.log(roundStateData);
    return roundStateData
}

export async function getPlayerRoundState() {
    let playerRoundStateInfo = await connection.getAccountInfo(playerState);
    let playerRoundStateData = borsh.deserialize(playerRoundStateSchema, PlayerRoundState, playerRoundStateInfo?.data as Buffer);
    console.log(playerRoundStateData);
    return playerRoundStateData
}

export async function getTokenAccBalance(acc: PublicKey) {
    let balance = (await connection.getTokenAccountBalance(acc)).value;
    console.log(`${acc} has`, balance.uiAmount as any / LAMPORTS_PER_SOL, 'sol');
    return balance;
}

// ============================================================================= core

export async function prepareTestEnv() {
    console.log('// --------------------------------------- configure env')
    version = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
    console.log(`version is ${version}`);
    await getConnection();
    //create a fake Wrapped SOL mint
    wSolMint = await createMintAccount();
    //assigning community & p3d accounts to bob - pretend he's the leader of both
    wSolComAcc = await createAndFundTokenAccount(wSolMint, bobKp.publicKey);
    wSolP3dAcc = await createAndFundTokenAccount(wSolMint, bobKp.publicKey);
    //funding alice and bob with 100 fake Wrapped SOL
    wSolAliceAcc = await createAndFundTokenAccount(wSolMint, aliceKp.publicKey, 100 * LAMPORTS_PER_SOL);
    wSolBobAcc = await createAndFundTokenAccount(wSolMint, bobKp.publicKey, 100 * LAMPORTS_PER_SOL);
}

export async function initGame(
    roundInitTime?: number,
    roundIncTimePerKey?: number,
    roundMaxTime?: number,
) {
    console.log('// --------------------------------------- init game')
    //game state pda
    let stateBumpSeed;
    [gameState, stateBumpSeed] = await PublicKey.findProgramAddress(
        [Buffer.from(`game${version}`)],
        FOMO_PROG_ID,
    )
    console.log('game state pda is:', gameState.toBase58());

    //init game ix
    const data = Buffer.from(Uint8Array.of(0,
        ...new BN(version).toArray('le', 8),
        ...new BN(roundInitTime ? roundInitTime : ROUND_INIT_TIME)
            .toArray('le', 8),
        ...new BN(roundIncTimePerKey ? roundIncTimePerKey : ROUND_INC_TIME_PER_KEY)
            .toArray('le', 8),
        ...new BN(roundMaxTime ? roundMaxTime : ROUND_MAX_TIME)
            .toArray('le', 8),
    ));
    const initIx = new TransactionInstruction({
        keys: [
            {
                pubkey: gameCreatorKp.publicKey,
                isSigner: true,
                isWritable: false
            },
            {pubkey: gameState, isSigner: false, isWritable: true},
            {pubkey: wSolComAcc, isSigner: false, isWritable: false},
            {pubkey: wSolP3dAcc, isSigner: false, isWritable: false},
            {pubkey: wSolMint.publicKey, isSigner: false, isWritable: false},
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false
            },
        ],
        programId: FOMO_PROG_ID,
        data,
    });
    await prepareAndSendTx([initIx], [gameCreatorKp]);
}

export async function initRound(round_id: number) {
    round = round_id;
    console.log(`// --------------------------------------- init round ${round}`)
    let roundBumpSeed, potBumpSeed;
    //round state pda
    [roundState, roundBumpSeed] = await PublicKey.findProgramAddress(
        [Buffer.from(`round${round}${version}`)],
        FOMO_PROG_ID,
    )
    console.log('round state pda is:', roundState.toBase58());

    //pot pda
    [wSolPot, potBumpSeed] = await PublicKey.findProgramAddress(
        [Buffer.from(`pot${round}${version}`)],
        FOMO_PROG_ID,
    )
    console.log('round pot pda is:', wSolPot.toBase58());

    //keys
    let keys = [
        {pubkey: gameCreatorKp.publicKey, isSigner: true, isWritable: false},
        {pubkey: gameState, isSigner: false, isWritable: true},
        {pubkey: roundState, isSigner: false, isWritable: true},
        {pubkey: wSolPot, isSigner: false, isWritable: true},
        {pubkey: wSolMint.publicKey, isSigner: false, isWritable: false},
        {pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false},
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false
        },
        {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
    ];
    if (round_id > 1) {
        let [prevRoundState, prevRoundBump] = await PublicKey.findProgramAddress(
            [Buffer.from(`round${round - 1}${version}`)],
            FOMO_PROG_ID,
        )
        let [prevWSolPot, prevWSolBump] = await PublicKey.findProgramAddress(
            [Buffer.from(`pot${round - 1}${version}`)],
            FOMO_PROG_ID,
        )
        keys.push({pubkey: prevRoundState, isSigner: false, isWritable: true});
        keys.push({pubkey: prevWSolPot, isSigner: false, isWritable: true});
    }

    //init round ix
    const data = Buffer.from(Uint8Array.of(1));
    const initRoundIx = new TransactionInstruction({
        keys,
        programId: FOMO_PROG_ID,
        data,
    });
    await prepareAndSendTx([initRoundIx], [gameCreatorKp]);
}

export async function purchaseKeys(
    buyer: Keypair,
    buyerTokenAcc: PublicKey,
    amountSol: number,
    addNewAff: (PublicKey | null) = null,
) {
    console.log('// --------------------------------------- purchase keys')
    let bump;
    //player-round state pda
    [playerState, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(`pr${buyer.publicKey.toBase58().substring(0, 12)}${round}${version}`)],
        FOMO_PROG_ID,
    )
    console.log('player-round state pda is:', playerState.toBase58());

    //keys
    let keys = [
        {pubkey: buyer.publicKey, isSigner: true, isWritable: false},
        {pubkey: gameState, isSigner: false, isWritable: false},
        {pubkey: roundState, isSigner: false, isWritable: true},
        {pubkey: playerState, isSigner: false, isWritable: true},
        {pubkey: wSolPot, isSigner: false, isWritable: true},
        {pubkey: buyerTokenAcc, isSigner: false, isWritable: true},
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false
        },
        {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
    ];
    if (addNewAff) {
        let [newAffRoundState, affBump] = await PublicKey.findProgramAddress(
            [Buffer.from(`pr${addNewAff.toBase58().substring(0, 12)}${round}${version}`)],
            FOMO_PROG_ID,
        )
        console.log('affiliate pda is:', newAffRoundState.toBase58());
        keys.push({pubkey: newAffRoundState, isSigner: false, isWritable: true})
        keys.push({pubkey: addNewAff, isSigner: false, isWritable: false})
    }

    //init round ix
    const data = Buffer.from(Uint8Array.of(2,
        ...new BN(amountSol * LAMPORTS_PER_SOL).toArray('le', 16), //1 sol
        ...new BN(1).toArray('le', 1), //team bear
    ));
    const purchaseKeysIx = new TransactionInstruction({
        keys,
        programId: FOMO_PROG_ID,
        data,
    });
    await prepareAndSendTx([purchaseKeysIx], [buyer]);
}

export async function withdrawSol(player: Keypair, playerTokenAcc: PublicKey) {
    console.log('// --------------------------------------- withdraw sol')
    const data = Buffer.from(Uint8Array.of(3, ...new BN(round).toArray('le', 8)));
    const withdrawSolIx = new TransactionInstruction({
        keys: [
            {pubkey: player.publicKey, isSigner: true, isWritable: false},
            {pubkey: gameState, isSigner: false, isWritable: false},
            {pubkey: roundState, isSigner: false, isWritable: false},
            {pubkey: playerState, isSigner: false, isWritable: true},
            {pubkey: wSolPot, isSigner: false, isWritable: true},
            {pubkey: playerTokenAcc, isSigner: false, isWritable: true},
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false
            },
            {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
        ],
        programId: FOMO_PROG_ID,
        data,
    });
    await prepareAndSendTx([withdrawSolIx], [player]);
}

export async function endRound() {
    console.log(`// --------------------------------------- end round ${round}`)
    const data = Buffer.from(Uint8Array.of(4));
    const endRoundIx = new TransactionInstruction({
        keys: [
            {pubkey: gameState, isSigner: false, isWritable: false},
            {pubkey: roundState, isSigner: false, isWritable: true},
            {pubkey: playerState, isSigner: false, isWritable: true},
        ],
        programId: FOMO_PROG_ID,
        data: data,
    });
    await prepareAndSendTx([endRoundIx], [gameCreatorKp]);
}

export async function withdrawCom() {
    console.log('// --------------------------------------- withdraw community funds')
    const data = Buffer.from(Uint8Array.of(5, ...new BN(round).toArray('le', 8)));
    const withdrawComIx = new TransactionInstruction({
        keys: [
            {pubkey: gameState, isSigner: false, isWritable: false},
            {pubkey: roundState, isSigner: false, isWritable: true},
            {pubkey: wSolPot, isSigner: false, isWritable: true},
            {pubkey: wSolComAcc, isSigner: false, isWritable: true},
            {pubkey: bobKp.publicKey, isSigner: true, isWritable: false},
            {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
        ],
        programId: FOMO_PROG_ID,
        data: data,
    });
    await prepareAndSendTx([withdrawComIx], [bobKp]);
}

