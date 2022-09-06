/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as splToken from '@solana/spl-token'
import * as beet from '@metaplex-foundation/beet'
import * as web3 from '@solana/web3.js'

/**
 * @category Instructions
 * @category CloseAccount
 * @category generated
 */
export const CloseAccountStruct = new beet.BeetArgsStruct<{
  instructionDiscriminator: number
}>([['instructionDiscriminator', beet.u8]], 'CloseAccountInstructionArgs')
/**
 * Accounts required by the _CloseAccount_ instruction
 *
 * @property [_writable_] account
 * @property [_writable_] destination
 * @property [] mint
 * @property [**signer**] owner
 * @property [**signer**] upstreamAuthority
 * @category Instructions
 * @category CloseAccount
 * @category generated
 */
export type CloseAccountInstructionAccounts = {
  account: web3.PublicKey
  destination: web3.PublicKey
  mint: web3.PublicKey
  owner: web3.PublicKey
  upstreamAuthority: web3.PublicKey
  tokenProgram?: web3.PublicKey
}

export const closeAccountInstructionDiscriminator = 5

/**
 * Creates a _CloseAccount_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @category Instructions
 * @category CloseAccount
 * @category generated
 */
export function createCloseAccountInstruction(
  accounts: CloseAccountInstructionAccounts,
  programId = new web3.PublicKey('PTxTEZXSadZ39at9G3hdXyYkKfyohTG3gCfNuSVnq4K')
) {
  const [data] = CloseAccountStruct.serialize({
    instructionDiscriminator: closeAccountInstructionDiscriminator,
  })
  const keys: web3.AccountMeta[] = [
    {
      pubkey: accounts.account,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: accounts.destination,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: accounts.mint,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: accounts.owner,
      isWritable: false,
      isSigner: true,
    },
    {
      pubkey: accounts.upstreamAuthority,
      isWritable: false,
      isSigner: true,
    },
    {
      pubkey: accounts.tokenProgram ?? splToken.TOKEN_PROGRAM_ID,
      isWritable: false,
      isSigner: false,
    },
  ]

  const ix = new web3.TransactionInstruction({
    programId,
    keys,
    data,
  })
  return ix
}
