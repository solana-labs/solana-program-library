import { PublicKey } from '@solana/web3.js';
import BigNumber from 'bignumber.js';

// export const LENDING_PROGRAM_ID = new PublicKey('6TvznH3B2e3p2mbhufNBpgSrLx6UkgvxtVQvopEZ2kuH'); //<-- existing
export const LENDING_PROGRAM_ID = new PublicKey('AhTXZQVzdtZjbUwMYhti1EggUx778n72kmgP6DT6xURY'); //<-- new

/** @internal */
// export const ORACLE_PROGRAM_ID = new PublicKey('5mkqGkkWSaSk2NL9p4XptwEQu4d5jFTJiurbbzdqYexF'); //<-- existing
export const ORACLE_PROGRAM_ID = new PublicKey('gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s'); //<-- new

/** @internal */
export const WAD = new BigNumber('1e+18');
