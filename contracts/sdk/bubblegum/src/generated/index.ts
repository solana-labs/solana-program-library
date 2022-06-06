import { PublicKey } from '@solana/web3.js'
export * from './accounts'
export * from './instructions'
export * from './types'

/**
 * Program address
 *
 * @category constants
 * @category generated
 */
export const PROGRAM_ADDRESS = 'BGUMzZr2wWfD2yzrXFEWTK2HbdYhqQCP2EZoPEkZBD6o'

/**
 * Program public key
 *
 * @category constants
 * @category generated
 */
export const PROGRAM_ID = new PublicKey(PROGRAM_ADDRESS)
