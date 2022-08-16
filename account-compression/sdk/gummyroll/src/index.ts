import { PublicKey } from '@solana/web3.js';
export * from './instructions';
export * from './accounts';
export * from './types';
export * from './utils';
export * from './convenience';

/**
 * Program address
 *
 * @category constants
 * @category generated
 */
export const PROGRAM_ADDRESS = 'GRoLLzvxpxxu2PGNJMMeZPyMxjAUH9pKqxGXV9DGiceU'

/**
 * Program public key
 *
 * @category constants
 * @category generated
 */
export const PROGRAM_ID = new PublicKey(PROGRAM_ADDRESS)
