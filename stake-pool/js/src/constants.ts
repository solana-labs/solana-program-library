import {Buffer} from 'buffer';
import {PublicKey} from '@solana/web3.js';
import {solToLamports} from './utils';

// Public key that identifies the SPL Stake Pool program.
export const STAKE_POOL_PROGRAM_ID = new PublicKey('SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy');

// Maximum number of validators to update during UpdateValidatorListBalance.
export const MAX_VALIDATORS_TO_UPDATE = 5;

// Seed used to derive transient stake accounts.
export const TRANSIENT_STAKE_SEED_PREFIX = Buffer.from('transient');

export const MIN_STAKE_BALANCE = solToLamports(0.001);

export const STAKE_STATE_LEN = 200;
