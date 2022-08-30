import { PublicKey } from '@solana/web3.js'
export * from './errors'
export * from './instructions'
export * from './types'

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
