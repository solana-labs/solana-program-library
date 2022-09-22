import { PublicKey } from '@solana/web3.js'
export * from './accounts'
export * from './errors'
export * from './instructions'
export * from './types'

/**
 * Program address
 *
 * @category constants
 * @category generated
 */
export const PROGRAM_ADDRESS = 'GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw'

/**
 * Program public key
 *
 * @category constants
 * @category generated
 */
export const PROGRAM_ID = new PublicKey(PROGRAM_ADDRESS)
