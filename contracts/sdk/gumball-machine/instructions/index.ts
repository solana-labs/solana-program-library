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
import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    NATIVE_MINT
  } from "@solana/spl-token";
import {
  GumballMachine,
  InitGumballMachineProps
} from '../types';
import {
  InitializeGumballMachineInstructionArgs,
  UpdateHeaderMetadataInstructionArgs,
  UpdateConfigLinesInstructionArgs,
  createInitializeGumballMachineInstruction,
  createDispenseNftSolInstruction,
  createUpdateHeaderMetadataInstruction,
  createDestroyInstruction,
  createUpdateConfigLinesInstruction,
  createDispenseNftTokenInstruction
} from "../src/generated";
import {
    getWillyWonkaPDAKey,
    getBubblegumAuthorityPDAKey
} from '../utils';

/**
 * Client side function to faciliate the creation of instructions for: initialize_gumball_machine
 * Handles the creation of merkle roll + gumball machine accounts and the initialization of the gumball machine header
 * with props from InitializeGumballMachineInstructionArgs, -> see ../src/generated/instructions/initializeGumballMachine.ts for details
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
): Promise<[TransactionInstruction]> {
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

    const willyWonkaPDAKey = await getWillyWonkaPDAKey(gumballMachineAcctKeypair.publicKey, gumballMachine.programId);
    const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDAKey(merkleRollKeypair.publicKey, bubblegumProgramId);

    const initGumballMachineInstr = createInitializeGumballMachineInstruction(
      {
        gumballMachine: gumballMachineAcctKeypair.publicKey,
        creator: payer.publicKey,
        mint,
        willyWonka: willyWonkaPDAKey,
        bubblegumAuthority: bubblegumAuthorityPDAKey,
        gummyroll: gummyrollProgramId,
        merkleSlab: merkleRollKeypair.publicKey,
        bubblegum: bubblegumProgramId
      },
      gumballMachineInitArgs
    );
    // initGumballMachineInstr.keys maybe initGumballMachineInstr.keys[].isSigner = true
    return [allocGumballMachineAcctInstr, allocMerkleRollAcctInstr, initGumballMachineInstr];
}

/**
 * Client side function to create instruction for: update_header_metadata
 * Enables the gumball machine authority to update config parameters in the GumballMachine header
 * */ 
export function createUpdateHeaderMetadataIx(
  authority: Keypair,
  gumballMachineAcctKey: PublicKey,
  updateHeaderArgs: UpdateHeaderMetadataInstructionArgs,
): TransactionInstruction {
  const updateHeaderMetadataInstr = createUpdateHeaderMetadataInstruction(
    {
      gumballMachine: gumballMachineAcctKey,
      authority: authority.publicKey
    },
    updateHeaderArgs
  );
  return updateHeaderMetadataInstr;
}

/**
 * Client side function to create instruction: dispense_nft_sol.
 * Enables payer to purchase a compressed NFT from a project seeking SOL
 * */
export async function createDispenseNFTForSolIx(
    numNFTs: BN,
    payer: Keypair,
    receiver: PublicKey,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    noncePDAKey: PublicKey,
    gummyrollProgramId: PublicKey,
    bubblegumProgramId: PublicKey,
    gumballMachine: Program<GumballMachine>,
  ): Promise<TransactionInstruction> {
    const willyWonkaPDAKey = await getWillyWonkaPDAKey(gumballMachineAcctKeypair.publicKey, gumballMachine.programId);
    const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDAKey(merkleRollKeypair.publicKey, bubblegumProgramId);
    const dispenseInstr = createDispenseNftSolInstruction(
          {
              gumballMachine: gumballMachineAcctKeypair.publicKey,
              payer: payer.publicKey,
              receiver: receiver,
              //systemProgram: SystemProgram.programId,
              willyWonka: willyWonkaPDAKey,
              recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
              instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
              bubblegumAuthority: bubblegumAuthorityPDAKey,
              nonce: noncePDAKey,
              gummyroll: gummyrollProgramId,
              merkleSlab: merkleRollKeypair.publicKey,
              bubblegum: bubblegumProgramId
          },
          {
            numItems: numNFTs
          }
    );
    return dispenseInstr;
}

/**
 * Client side function to create instruction: dispense_nft_token.
 * Enables payer to purchase a compressed NFT from a project seeking SPL tokens
 * */
export async function createDispenseNFTForTokensIx(
    numNFTs: BN,
    payer: Keypair,
    payerTokens: PublicKey,
    receiver: PublicKey,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    noncePDAKey: PublicKey,
    gummyrollProgramId: PublicKey,
    bubblegumProgramId: PublicKey,
    gumballMachine: Program<GumballMachine>,
): Promise<TransactionInstruction> {
    const willyWonkaPDAKey = await getWillyWonkaPDAKey(gumballMachineAcctKeypair.publicKey, gumballMachine.programId);
    const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDAKey(merkleRollKeypair.publicKey, bubblegumProgramId);
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
          nonce: noncePDAKey,
          gummyroll: gummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          bubblegum: bubblegumProgramId
        },
        {
          numItems: numNFTs
        }
    );
    return dispenseInstr;
}

/**
 * Client side function to create instruction: add_config_lines.
 * Enables gumballMachine authority to add config lines -> compressed NFTs that can be minted
 * */
/*export function createAddConfigLinesIx(
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    configLinesToAdd: Buffer,
    gumballMachine: Program<GumballMachine>
): TransactionInstruction {
    const addConfigLinesInstr = gumballMachine.instruction.addConfigLines(
        configLinesToAdd,
        {
            accounts: {
                gumballMachine: gumballMachineAcctKey,
                authority: authority.publicKey
            },
            signers: [authority]
        }
    )
    return addConfigLinesInstr;
}*/

/**
 * Client side function to create instruction: destroy
 * Enables authority for a GumballMachine to pull all lamports out of their GumballMachine account
 * effectively "destroying" the GumballMachine for use and returning their funds
 * */
export function createDestroyGumballMachineIx(
    gumballMachineAcctKeypair: Keypair,
    authorityKeypair: Keypair
  ): TransactionInstruction {
    const destroyInstr = createDestroyInstruction(
        {
            gumballMachine: gumballMachineAcctKeypair.publicKey,
            authority: authorityKeypair.publicKey
        }
    );
    return destroyInstr;
}

/**
 * Client side function to create instruction: update_config_lines
 * Enables authority to update the data stored in certain config lines which have already been added to their GumballMachine
 * */
export function createUpdateConfigLinesIx(
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    args: UpdateConfigLinesInstructionArgs
): TransactionInstruction {
    const updateConfigLinesInstr = createUpdateConfigLinesInstruction(
        {
          gumballMachine: gumballMachineAcctKey,
          authority: authority.publicKey
        },
        args
    );
    return updateConfigLinesInstr;
}