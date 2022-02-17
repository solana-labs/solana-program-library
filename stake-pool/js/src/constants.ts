import {Buffer} from 'buffer';
import {PublicKey} from '@solana/web3.js';
import {solToLamports} from './utils';

export const TRANSIENT_STAKE_SEED_PREFIX = Buffer.from('transient');

export const STAKE_POOL_PROGRAM_ID = new PublicKey(
  'SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy',
);

export const MIN_STAKE_BALANCE = solToLamports(0.001);
