import {
  PublicKey,
  Connection,
  Keypair,
  clusterApiUrl,
  SystemProgram
} from '@solana/web3.js';
import {assert, expect} from 'chai';
import {
  DepositSolParams,
  StakePoolProgram,
  STAKE_POOL_INSTRUCTION_LAYOUTS
} from '../src/stakepool-program';
import {decodeData} from '../src/copied-from-solana-web3/instruction';
import {getFirstStakePoolAccount} from "./utils";
import {depositSol} from "../src";

describe('StakePoolProgram', async () => {

  const connection = new Connection(
    clusterApiUrl('devnet'),
    'confirmed',
  );

  const STAKE_POOL_PROGRAM_ADDR = new PublicKey(
    'SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy',
  );

  // it('withdrawStake', async () => {
  //     const stakePool = await getFirstStakePoolAccount(connection, STAKE_POOL_PROGRAM_ADDR);
  // });

  it('depositSol', async () => {
    const fromPubkey = Keypair.generate().publicKey;
    const lamports = 1;
    const res = await depositSol(
      connection,
      STAKE_POOL_PROGRAM_ADDR,
      fromPubkey,
      lamports,
    );
    console.log(res);
  });

  it('depositSolInstruction', async () => {

    const payload: DepositSolParams = {
      stakePoolPubkey: STAKE_POOL_PROGRAM_ADDR,
      withdrawAuthority: Keypair.generate().publicKey,
      reserveStakeAccount: Keypair.generate().publicKey,
      lamportsFrom: Keypair.generate().publicKey,
      poolTokensTo: Keypair.generate().publicKey,
      managerFeeAccount: Keypair.generate().publicKey,
      referrerPoolTokensAccount: Keypair.generate().publicKey,
      poolMint: Keypair.generate().publicKey,
      lamports: 99999,
    };

    const instruction = await StakePoolProgram.depositSolInstruction(payload);

    expect(instruction.keys).is.length(10);

    assert.equal(instruction.keys[0].pubkey.toBase58(), payload.stakePoolPubkey.toBase58());
    assert.equal(instruction.keys[1].pubkey.toBase58(), payload.withdrawAuthority.toBase58());
    assert.equal(instruction.keys[3].pubkey.toBase58(), payload.lamportsFrom.toBase58());
    assert.equal(instruction.keys[4].pubkey.toBase58(), payload.poolTokensTo.toBase58());
    assert.equal(instruction.keys[5].pubkey.toBase58(), payload.managerFeeAccount.toBase58());
    assert.equal(instruction.keys[6].pubkey.toBase58(), payload.referrerPoolTokensAccount.toBase58());
    assert.equal(instruction.keys[8].pubkey.toBase58(), SystemProgram.programId.toBase58());
    assert.equal(instruction.keys[9].pubkey.toBase58(), StakePoolProgram.tokenProgramId.toBase58());

    const decodedData = decodeData(STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol, instruction.data);

    assert.equal(decodedData.instruction, STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol.index);
    assert.equal(decodedData.lamports, payload.lamports);

    payload.depositAuthority = Keypair.generate().publicKey;

    const instruction2 = await StakePoolProgram.depositSolInstruction(payload);

    expect(instruction2.keys).is.length(11);
    assert.equal(instruction2.keys[10].pubkey.toBase58(), payload.depositAuthority.toBase58());

    return true;
  });

});
