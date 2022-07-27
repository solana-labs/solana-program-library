import { Provider } from "@project-serum/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Connection as web3Connection,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_SLOT_HASHES_PUBKEY,
  TransactionInstruction,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import {
  createInitializeIndicesInstructions
} from "../instructions";
import { execute } from "../../utils";

/**
 * 
 * @param maxItems - this must be the max_items already set in the gumball machine header for this function to work correctly
 * @param authority 
 * @param gumballMachine 
 */
export async function initializeGumballMachineIndices(
  provider: Provider,
  maxItems: number,
  authority: Keypair,
  gumballMachine: PublicKey,
  verbose: boolean = false
) {
  let initializeIndexInstructions = createInitializeIndicesInstructions(maxItems, authority.publicKey, gumballMachine);
  for (let i = 0; i < initializeIndexInstructions.length; i++) {
    const instructions = [ComputeBudgetProgram.requestUnits({ units: 1.4e6, additionalFee: 0 }), initializeIndexInstructions[i]];
    await execute(provider, instructions, [authority], true, verbose);
  }
}
