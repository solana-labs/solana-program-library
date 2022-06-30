import { BN, Program } from "@project-serum/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Connection as web3Connection,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_SLOT_HASHES_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";
import { GumballMachine } from "../types";
import { getBubblegumAuthorityPDA } from "../../bubblegum/src/convenience";
import {
  InitializeGumballMachineInstructionArgs,
  createInitializeGumballMachineInstruction,
  createDispenseNftSolInstruction,
  createDispenseNftTokenInstruction,
} from "../src/generated";
import { getWillyWonkaPDAKey } from "../utils";
import { CANDY_WRAPPER_PROGRAM_ID } from "../../utils";

/**
 * Wrapper on top of Solita's createInitializeGumballMachineInstruction
 * Produces a series of instructions to create the merkle roll + gumball machine accounts and initialize gumball machine
 * */
export async function createInitializeGumballMachineIxs(
  payer: Keypair,
  gumballMachineAcctKeypair: Keypair,
  gumballMachineAcctSize: number,
  merkleRollKeypair: Keypair,
  merkleRollAccountSize: number,
  gumballMachineInitArgs: InitializeGumballMachineInstructionArgs,
  mint: PublicKey,
  gummyrollProgramId: PublicKey,
  bubblegumProgramId: PublicKey,
  gumballMachine: Program<GumballMachine>
): Promise<TransactionInstruction[]> {
  const allocGumballMachineAcctInstr = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: gumballMachineAcctKeypair.publicKey,
    lamports:
      await gumballMachine.provider.connection.getMinimumBalanceForRentExemption(
        gumballMachineAcctSize
      ),
    space: gumballMachineAcctSize,
    programId: gumballMachine.programId,
  });

  const allocMerkleRollAcctInstr = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: merkleRollKeypair.publicKey,
    lamports:
      await gumballMachine.provider.connection.getMinimumBalanceForRentExemption(
        merkleRollAccountSize
      ),
    space: merkleRollAccountSize,
    programId: gummyrollProgramId,
  });

  const willyWonkaPDAKey = await getWillyWonkaPDAKey(
    gumballMachineAcctKeypair.publicKey,
    gumballMachine.programId
  );
  const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDA(
    merkleRollKeypair.publicKey,
  );

  const initGumballMachineInstr = createInitializeGumballMachineInstruction(
    {
      gumballMachine: gumballMachineAcctKeypair.publicKey,
      creator: payer.publicKey,
      mint,
      willyWonka: willyWonkaPDAKey,
      bubblegumAuthority: bubblegumAuthorityPDAKey,
      candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
      gummyroll: gummyrollProgramId,
      merkleSlab: merkleRollKeypair.publicKey,
      bubblegum: bubblegumProgramId,
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
 * Wrapper on top of Solita's createDispenseNftSolInstruction. Automatically fetches necessary PDA keys for instruction
 * */
export async function createDispenseNFTForSolIx(
  numNFTs: BN,
  payer: Keypair,
  receiver: PublicKey,
  gumballMachineAcctKeypair: Keypair,
  merkleRollKeypair: Keypair,
  gummyrollProgramId: PublicKey,
  bubblegumProgramId: PublicKey,
  gumballMachine: Program<GumballMachine>
): Promise<TransactionInstruction> {
  const willyWonkaPDAKey = await getWillyWonkaPDAKey(
    gumballMachineAcctKeypair.publicKey,
    gumballMachine.programId
  );
  const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDA(
    merkleRollKeypair.publicKey,
  );
  const dispenseInstr = createDispenseNftSolInstruction(
    {
      gumballMachine: gumballMachineAcctKeypair.publicKey,
      payer: payer.publicKey,
      receiver: receiver,
      willyWonka: willyWonkaPDAKey,
      recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
      instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
      bubblegumAuthority: bubblegumAuthorityPDAKey,
      candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
      gummyroll: gummyrollProgramId,
      merkleSlab: merkleRollKeypair.publicKey,
      bubblegum: bubblegumProgramId,
    },
    {
      numItems: numNFTs,
    }
  );
  return dispenseInstr;
}

/**
 * Wrapper on top of Solita's createDispenseNftTokenInstruction. Automatically fetches necessary PDA keys for instruction
 * */
export async function createDispenseNFTForTokensIx(
  numNFTs: BN,
  payer: Keypair,
  payerTokens: PublicKey,
  receiver: PublicKey,
  gumballMachineAcctKeypair: Keypair,
  merkleRollKeypair: Keypair,
  gummyrollProgramId: PublicKey,
  bubblegumProgramId: PublicKey,
  gumballMachine: Program<GumballMachine>,
): Promise<TransactionInstruction> {
  const willyWonkaPDAKey = await getWillyWonkaPDAKey(
    gumballMachineAcctKeypair.publicKey,
    gumballMachine.programId
  );
  const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDA(
    merkleRollKeypair.publicKey,
  );
  const dispenseInstr = createDispenseNftTokenInstruction(
    {
      gumballMachine: gumballMachineAcctKeypair.publicKey,
      payer: payer.publicKey,
      payerTokens,
      receiver,
      willyWonka: willyWonkaPDAKey,
      recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
      instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
      bubblegumAuthority: bubblegumAuthorityPDAKey,
      candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
      gummyroll: gummyrollProgramId,
      merkleSlab: merkleRollKeypair.publicKey,
      bubblegum: bubblegumProgramId,
    },
    {
      numItems: numNFTs,
    }
  );
  return dispenseInstr;
}
