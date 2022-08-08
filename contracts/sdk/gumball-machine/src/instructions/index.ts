import {
  PublicKey,
  Keypair,
  SystemProgram,
  Connection as web3Connection,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_SLOT_HASHES_PUBKEY,
  TransactionInstruction,
  Connection,
} from "@solana/web3.js";
import {
  InitializeGumballMachineInstructionArgs,
  createInitializeGumballMachineInstruction,
  createDispenseNftSolInstruction,
  createDispenseNftTokenInstruction,
  DispenseNftSolInstructionArgs,
  DispenseNftTokenInstructionArgs,
  createInitializeIndicesChunkInstruction,
  PROGRAM_ID as GUMBALL_MACHINE_PROGRAM_ID
} from "../generated";
import { getWillyWonkaPDAKey } from "../utils";
import { CANDY_WRAPPER_PROGRAM_ID } from "@sorend-solana/utils";
import { PROGRAM_ID as BUBBLEGUM_MACHINE_PROGRAM_ID, getBubblegumAuthorityPDA } from "@sorend-solana/bubblegum";
import { PROGRAM_ID as GUMMYROLL_MACHINE_PROGRAM_ID } from "@sorend-solana/gummyroll";

/**
 * Wrapper on top of Solita's createInitializeGumballMachineInstruction
 * Produces a series of instructions to create the merkle roll + gumball machine accounts and initialize gumball machine
 * */
export async function createInitializeGumballMachineIxs(
  payerPublicKey: PublicKey,
  gumballMachinePublicKey: PublicKey,
  gumballMachineAcctSize: number,
  merkleRollPublicKey: PublicKey,
  merkleRollAccountSize: number,
  gumballMachineInitArgs: InitializeGumballMachineInstructionArgs,
  mint: PublicKey,
  connection: Connection
): Promise<TransactionInstruction[]> {
  const allocGumballMachineAcctInstr = SystemProgram.createAccount({
    fromPubkey: payerPublicKey,
    newAccountPubkey: gumballMachinePublicKey,
    lamports:
      await connection.getMinimumBalanceForRentExemption(
        gumballMachineAcctSize
      ),
    space: gumballMachineAcctSize,
    programId: GUMBALL_MACHINE_PROGRAM_ID,
  });

  const allocMerkleRollAcctInstr = SystemProgram.createAccount({
    fromPubkey: payerPublicKey,
    newAccountPubkey: merkleRollPublicKey,
    lamports:
      await connection.getMinimumBalanceForRentExemption(
        merkleRollAccountSize
      ),
    space: merkleRollAccountSize,
    programId: GUMMYROLL_MACHINE_PROGRAM_ID,
  });

  const willyWonkaPDAKey = await getWillyWonkaPDAKey(
    gumballMachinePublicKey,
    GUMBALL_MACHINE_PROGRAM_ID
  );
  const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDA(
    merkleRollPublicKey,
  );

  const initGumballMachineInstr = createInitializeGumballMachineInstruction(
    {
      gumballMachine: gumballMachinePublicKey,
      payer: payerPublicKey,
      mint,
      willyWonka: willyWonkaPDAKey,
      bubblegumAuthority: bubblegumAuthorityPDAKey,
      candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
      gummyroll: GUMMYROLL_MACHINE_PROGRAM_ID,
      merkleSlab: merkleRollPublicKey,
      bubblegum: BUBBLEGUM_MACHINE_PROGRAM_ID,
    },
    gumballMachineInitArgs
  );
  return [
    allocGumballMachineAcctInstr,
    allocMerkleRollAcctInstr,
    initGumballMachineInstr,
  ];
}

/**
 * Wrapper to generate all instructions needed to initialize a gumball machine's indices. 
 * @notice Each instruction should be executed in its transaction to stay within Solana's compute limit
 */
export function createInitializeIndicesInstructions(
  maxItems: number,
  authority: PublicKey,
  gumballMachine: PublicKey
): TransactionInstruction[] {
  let smallestUnintializedInd = 0;
  let indexInitInstrs: TransactionInstruction[] = [];
  while (smallestUnintializedInd < maxItems) {
    let initIndexChunkInstr = createInitializeIndicesChunkInstruction(
      {
        authority,
        gumballMachine,
      }
    )
    indexInitInstrs.push(initIndexChunkInstr);
    smallestUnintializedInd = Math.min(smallestUnintializedInd + 250000, maxItems);
  }
  return indexInitInstrs;
}

/**
 * Wrapper on top of Solita's createDispenseNftSolInstruction. Automatically fetches necessary PDA keys for instruction
 * */
export async function createDispenseNFTForSolIx(
  args: DispenseNftSolInstructionArgs,
  payer: PublicKey,
  receiver: PublicKey,
  gumballMachinePubkey: PublicKey,
  merkleRollPubkey: PublicKey
): Promise<TransactionInstruction> {
  const willyWonkaPDAKey = await getWillyWonkaPDAKey(gumballMachinePubkey, GUMBALL_MACHINE_PROGRAM_ID);
  const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDA(
    merkleRollPubkey,
  );
  const dispenseInstr = createDispenseNftSolInstruction(
    {
      gumballMachine: gumballMachinePubkey,
      payer,
      receiver: receiver,
      willyWonka: willyWonkaPDAKey,
      recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
      instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
      bubblegumAuthority: bubblegumAuthorityPDAKey,
      candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
      gummyroll: GUMMYROLL_MACHINE_PROGRAM_ID,
      merkleSlab: merkleRollPubkey,
      bubblegum: BUBBLEGUM_MACHINE_PROGRAM_ID,
    },
    args
  );
  return dispenseInstr;
}

/**
 * Wrapper on top of Solita's createDispenseNftTokenInstruction. Automatically fetches necessary PDA keys for instruction
 * */
export async function createDispenseNFTForTokensIx(
  args: DispenseNftTokenInstructionArgs,
  payer: PublicKey,
  payerTokens: PublicKey,
  receiver: PublicKey,
  gumballMachinePubkey: PublicKey,
  merkleRollPubkey: PublicKey
): Promise<TransactionInstruction> {
  const willyWonkaPDAKey = await getWillyWonkaPDAKey(
    gumballMachinePubkey,
    GUMBALL_MACHINE_PROGRAM_ID
  );
  const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDA(
    merkleRollPubkey,
  );
  const dispenseInstr = createDispenseNftTokenInstruction(
    {
      gumballMachine: gumballMachinePubkey,
      payer,
      payerTokens,
      receiver,
      willyWonka: willyWonkaPDAKey,
      recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
      instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
      bubblegumAuthority: bubblegumAuthorityPDAKey,
      candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
      gummyroll: GUMMYROLL_MACHINE_PROGRAM_ID,
      merkleSlab: merkleRollPubkey,
      bubblegum: BUBBLEGUM_MACHINE_PROGRAM_ID,
    },
    args
  );
  return dispenseInstr;
}
