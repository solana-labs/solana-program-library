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
 * @category InitializeAccount
 * @category generated
 */
export const InitializeAccountStruct = new beet.BeetArgsStruct<{
  instructionDiscriminator: number
}>([['instructionDiscriminator', beet.u8]], 'InitializeAccountInstructionArgs')
/**
 * Accounts required by the _InitializeAccount_ instruction
 *
 * @property [_writable_] account
 * @property [] owner
 * @property [_writable_, **signer**] payer
 * @property [**signer**] upstreamAuthority
 * @property [] mint
 * @property [] associatedTokenProgram Associated Token program
 * @category Instructions
 * @category InitializeAccount
 * @category generated
 */
export type InitializeAccountInstructionAccounts = {
  account: web3.PublicKey
  owner: web3.PublicKey
  payer: web3.PublicKey
  upstreamAuthority: web3.PublicKey
  mint: web3.PublicKey
  systemProgram?: web3.PublicKey
  rent?: web3.PublicKey
  associatedTokenProgram: web3.PublicKey
  tokenProgram?: web3.PublicKey
}

export const initializeAccountInstructionDiscriminator = 1

/**
 * Creates a _InitializeAccount_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @category Instructions
 * @category InitializeAccount
 * @category generated
 */
export function createInitializeAccountInstruction(
  accounts: InitializeAccountInstructionAccounts,
  programId = new web3.PublicKey('PTxTEZXSadZ39at9G3hdXyYkKfyohTG3gCfNuSVnq4K')
) {
  const [data] = InitializeAccountStruct.serialize({
    instructionDiscriminator: initializeAccountInstructionDiscriminator,
  })
  const keys: web3.AccountMeta[] = [
    {
      pubkey: accounts.account,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: accounts.owner,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: accounts.payer,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: accounts.upstreamAuthority,
      isWritable: false,
      isSigner: true,
    },
    {
      pubkey: accounts.mint,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: accounts.systemProgram ?? web3.SystemProgram.programId,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: accounts.rent ?? web3.SYSVAR_RENT_PUBKEY,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: accounts.associatedTokenProgram,
      isWritable: false,
      isSigner: false,
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
