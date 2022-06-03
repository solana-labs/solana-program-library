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
    getWillyWonkaPDAKey,
    getBubblegumAuthorityPDAKey
} from '../utils';

/**
 * Client side function to faciliate the creation of instructions for: initialize_gumball_machine
 * Handles the creation of merkle roll + gumball machine accounts and the initialization of the gumball machine header
 * with props from InitGumballMachineProps -> see ../types/index.ts for details
 * */
export async function createInitializeGumballMachineIxs(
  payer: Keypair,
  gumballMachineAcctKeypair: Keypair,
  gumballMachineAcctSize: number,
  merkleRollKeypair: Keypair,
  merkleRollAccountSize: number,
  desiredGumballMachineHeader: InitGumballMachineProps,
  maxDepth: number,
  maxBufferSize: number,
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

    const initGumballMachineInstr = gumballMachine.instruction.initializeGumballMachine(
      maxDepth,
      maxBufferSize,
      desiredGumballMachineHeader.urlBase, 
      desiredGumballMachineHeader.nameBase,
      desiredGumballMachineHeader.symbol,
      desiredGumballMachineHeader.sellerFeeBasisPoints,
      desiredGumballMachineHeader.isMutable,
      desiredGumballMachineHeader.retainAuthority,
      desiredGumballMachineHeader.price,
      desiredGumballMachineHeader.goLiveDate,
      desiredGumballMachineHeader.botWallet,
      desiredGumballMachineHeader.receiver,
      desiredGumballMachineHeader.authority, 
      desiredGumballMachineHeader.collectionKey,
      desiredGumballMachineHeader.extensionLen,
      desiredGumballMachineHeader.maxMintSize,
      desiredGumballMachineHeader.maxItems,
      {
        accounts: {
          gumballMachine: gumballMachineAcctKeypair.publicKey,
          creator: payer.publicKey,
          mint: desiredGumballMachineHeader.mint,
          willyWonka: willyWonkaPDAKey,
          bubblegumAuthority: bubblegumAuthorityPDAKey,
          gummyroll: gummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          bubblegum: bubblegumProgramId
        },
        signers: [payer],
      }
    );
    return [allocGumballMachineAcctInstr, allocMerkleRollAcctInstr, initGumballMachineInstr];
}

/**
 * Client side function to create instruction for: update_header_metadata
 * Enables the gumball machine authority to update config parameters in the GumballMachine header
 * */ 
export function createUpdateHeaderMetadataIx(
  authority: Keypair,
  gumballMachineAcctKey: PublicKey,
  newHeader: InitGumballMachineProps,
  gumballMachine: Program<GumballMachine>
): TransactionInstruction {
  const updateHeaderMetadataInstr = gumballMachine.instruction.updateHeaderMetadata(
    newHeader.urlBase,
    newHeader.nameBase,
    newHeader.symbol,
    newHeader.sellerFeeBasisPoints,
    newHeader.isMutable,
    newHeader.retainAuthority,
    newHeader.price,
    newHeader.goLiveDate,
    newHeader.botWallet,
    newHeader.authority, 
    newHeader.maxMintSize,
    {
      accounts: {
        gumballMachine: gumballMachineAcctKey,
        authority: authority.publicKey
      },
      signers: [authority]
    }
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
    const dispenseInstr = gumballMachine.instruction.dispenseNftSol(
        numNFTs,
        {
            accounts: {
                gumballMachine: gumballMachineAcctKeypair.publicKey,
                payer: payer.publicKey,
                receiver: receiver,
                systemProgram: SystemProgram.programId,
                willyWonka: willyWonkaPDAKey,
                recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
                instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
                bubblegumAuthority: bubblegumAuthorityPDAKey,
                nonce: noncePDAKey,
                gummyroll: gummyrollProgramId,
                merkleSlab: merkleRollKeypair.publicKey,
                bubblegum: bubblegumProgramId
            },
            signers: [payer]
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
    const dispenseInstr = gumballMachine.instruction.dispenseNftToken(
        numNFTs,
        {
            accounts: {
                gumballMachine: gumballMachineAcctKeypair.publicKey,
                payer: payer.publicKey,
                payerTokens,
                receiver,
                tokenProgram: TOKEN_PROGRAM_ID,
                willyWonka: willyWonkaPDAKey,
                recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
                instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
                bubblegumAuthority: bubblegumAuthorityPDAKey,
                nonce: noncePDAKey,
                gummyroll: gummyrollProgramId,
                merkleSlab: merkleRollKeypair.publicKey,
                bubblegum: bubblegumProgramId
            },
            signers: [payer]
        }
    );
    return dispenseInstr;
}

/**
 * Client side function to create instruction: add_config_lines.
 * Enables gumballMachine authority to add config lines -> compressed NFTs that can be minted
 * */
export function createAddConfigLinesIx(
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
}

/**
 * Client side function to create instruction: destroy
 * Enables authority for a GumballMachine to pull all lamports out of their GumballMachine account
 * effectively "destroying" the GumballMachine for use and returning their funds
 * */
export function createDestroyGumballMachineIx(
    gumballMachineAcctKeypair: Keypair,
    authorityKeypair: Keypair,
    gumballMachine: Program<GumballMachine>
  ): TransactionInstruction {
    const destroyInstr = gumballMachine.instruction.destroy(
        {
        accounts: {
            gumballMachine: gumballMachineAcctKeypair.publicKey,
            authority: authorityKeypair.publicKey
        },
        signers: [authorityKeypair]
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
    updatedConfigLines: Buffer,
    indexOfFirstLineToUpdate: BN,
    gumballMachine: Program<GumballMachine>
): TransactionInstruction {
    const updateConfigLinesInstr = gumballMachine.instruction.updateConfigLines(
        indexOfFirstLineToUpdate,
        updatedConfigLines,
        {
            accounts: {
            gumballMachine: gumballMachineAcctKey,
            authority: authority.publicKey
            },
            signers: [authority]
        }
    );
    return updateConfigLinesInstr;
}