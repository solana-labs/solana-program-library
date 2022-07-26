import * as anchor from "@project-serum/anchor";
import { CompiledInnerInstruction, CompiledInstruction, Context, Logs, PublicKey } from "@solana/web3.js";
import { readFileSync } from "fs";
import { Bubblegum } from "../../../target/types/bubblegum";
import { Gummyroll } from "../../../target/types/gummyroll";
import { GumballMachine } from "../../../target/types/gumball_machine";
import { NFTDatabaseConnection } from "../db";
import { parseBubblegumInstruction } from "./instruction/bubblegum";
import { parseBubblegumInnerInstructions } from "./innerInstruction/bubblegum";
import { Idl, IdlTypeDef } from '@project-serum/anchor/dist/cjs/idl';
import { IdlCoder } from '@project-serum/anchor/dist/cjs/coder/borsh/idl';
import { Layout } from "buffer-layout";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import {
  Creator,
  MetadataArgs,
  metadataArgsBeet,
  TokenProgramVersion,
  TokenStandard,
  PROGRAM_ID as BUBBLEGUM_PROGRAM_ID,
  getLeafAssetId
} from "@sorend-solana/bubblegum";
import { NewLeafEvent, LeafSchemaEvent } from "./ingester";
import { keccak_256 } from "js-sha3";
import * as beetSolana from '@metaplex-foundation/beet-solana';
import * as beet from '@metaplex-foundation/beet';

import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from "@sorend-solana/gummyroll";
import { PROGRAM_ID as GUMBALL_MACHINE_ID } from "@sorend-solana/gumball-machine";
import { CANDY_WRAPPER_PROGRAM_ID } from "@sorend-solana/utils";

export type ParserState = {
  Gummyroll: anchor.Program<Gummyroll>;
  Bubblegum: anchor.Program<Bubblegum>;
};

export type OptionalInfo = {
  txId: string;
  startSeq: number | null;
  endSeq: number | null;
};

/**
 * Example:
 * ```
 * let event = decodeEvent(dataString, Gummyroll.idl) ?? decodeEvent(dataString, Bubblegum.idl);
 * ```
 * @param data
 * @param idl
 * @returns
 */
export function decodeEvent(data: string, idl: anchor.Idl): anchor.Event | null {
  let eventCoder = new anchor.BorshEventCoder(idl);
  return eventCoder.decode(data);
}

export function loadProgram(
  provider: anchor.Provider,
  programId: PublicKey,
  idlPath: string
) {
  const IDL = JSON.parse(readFileSync(idlPath).toString());
  return new anchor.Program(IDL, programId, provider);
}

export enum ParseResult {
  Success,
  LogTruncated,
  TransactionError
};

function indexZippedInstruction(
  db: NFTDatabaseConnection,
  context: { txId: string, startSeq: number, endSeq: number },
  slot: number,
  parserState: ParserState,
  accountKeys: PublicKey[],
  zippedInstruction: ZippedInstruction,
) {
  const { instruction, innerInstructions } = zippedInstruction;
  const programId = accountKeys[instruction.programIdIndex];
  if (programId.equals(BUBBLEGUM_PROGRAM_ID)) {
    console.log("Found bubblegum");
    parseBubblegumInstruction(
      db,
      slot,
      parserState,
      context,
      accountKeys,
      instruction,
      innerInstructions
    );
  } else {
    if (innerInstructions.length) {
      parseBubblegumInnerInstructions(
        db,
        slot,
        parserState,
        context,
        accountKeys,
        innerInstructions[0].instructions,
      )
    }
  }
}

export function decodeEventInstructionData(
  idl: Idl,
  eventName: string,
  base58String: string,
) {
  const rawLayouts: [string, Layout<any>][] = idl.events.map((event) => {
    let eventTypeDef: IdlTypeDef = {
      name: event.name,
      type: {
        kind: "struct",
        fields: event.fields.map((f) => {
          return { name: f.name, type: f.type };
        }),
      },
    };
    return [event.name, IdlCoder.typeDefLayout(eventTypeDef, idl.types)];
  });
  const layouts = new Map(rawLayouts);
  const buffer = bs58.decode(base58String);
  const layout = layouts.get(eventName);
  if (!layout) {
    console.error("Could not find corresponding layout for event:", eventName);
  }
  const data = layout.decode(buffer);
  return { data, name: eventName };
}

export function destructureBubblegumMintAccounts(
  accountKeys: PublicKey[],
  instruction: CompiledInstruction
) {
  return {
    owner: accountKeys[instruction.accounts[4]],
    delegate: accountKeys[instruction.accounts[5]],
    merkleSlab: accountKeys[instruction.accounts[6]],
  }
}


type ZippedInstruction = {
  instructionIndex: number,
  instruction: CompiledInstruction,
  innerInstructions: CompiledInnerInstruction[],
}

/// Similar to `order_instructions` in `/nft_ingester/src/utils/instructions.rs`
function zipInstructions(
  instructions: CompiledInstruction[],
  innerInstructions: CompiledInnerInstruction[],
): ZippedInstruction[] {
  const zippedIxs: ZippedInstruction[] = [];
  let innerIxIndex = 0;
  const innerIxMap: Map<number, CompiledInnerInstruction> = new Map();
  for (const innerIx of innerInstructions) {
    innerIxMap.set(innerIx.index, innerIx);
  }
  for (const [instructionIndex, instruction] of instructions.entries()) {
    zippedIxs.push({
      instructionIndex,
      instruction,
      innerInstructions: innerIxMap.has(instructionIndex) ? [innerIxMap.get(instructionIndex)] : []
    })
  }
  return zippedIxs;
}

export function handleInstructionsAtomic(
  db: NFTDatabaseConnection,
  instructionInfo: {
    accountKeys: PublicKey[],
    instructions: CompiledInstruction[],
    innerInstructions: CompiledInnerInstruction[],
  },
  txId: string,
  context: Context,
  parsedState: ParserState,
  startSeq: number | null = null,
  endSeq: number | null = null
) {
  const { accountKeys, instructions, innerInstructions } = instructionInfo;

  const zippedInstructions = zipInstructions(instructions, innerInstructions);
  for (const zippedInstruction of zippedInstructions) {
    indexZippedInstruction(
      db,
      { txId, startSeq, endSeq },
      context.slot,
      parsedState,
      accountKeys,
      zippedInstruction,
    )
  }
}

export function loadPrograms(provider: anchor.Provider) {
  const Gummyroll = loadProgram(
    provider,
    GUMMYROLL_PROGRAM_ID,
    "target/idl/gummyroll.json"
  ) as anchor.Program<Gummyroll>;
  const Bubblegum = loadProgram(
    provider,
    BUBBLEGUM_PROGRAM_ID,
    "target/idl/bubblegum.json"
  ) as anchor.Program<Bubblegum>;
  const GumballMachine = loadProgram(
    provider,
    GUMBALL_MACHINE_ID,
    "target/idl/gumball_machine.json"
  ) as anchor.Program<GumballMachine>;
  return { Gummyroll, Bubblegum, GumballMachine };
}

export function hashMetadata(message: MetadataArgs) {
  // Todo: fix Solita - This is an issue with beet serializing complex enums
  message.tokenStandard = getTokenStandard(message.tokenStandard);
  message.tokenProgramVersion = getTokenProgramVersion(message.tokenProgramVersion);

  const [serialized, byteSize] = metadataArgsBeet.serialize(message);
  if (byteSize < 20) {
    console.log(serialized.length);
    console.error("Unable to serialize metadata args properly")
  }
  return digest(serialized)
}

type UnverifiedCreator = {
  address: PublicKey,
  share: number
};

export const unverifiedCreatorBeet = new beet.BeetArgsStruct<UnverifiedCreator>(
  [
    ['address', beetSolana.publicKey],
    ['share', beet.u8],
  ],
  'UnverifiedCreator'
)

export function hashCreators(creators: Creator[]) {
  const bytes = [];
  for (const creator of creators) {
    const unverifiedCreator = {
      address: creator.address,
      share: creator.share
    }
    const [buffer, _byteSize] = unverifiedCreatorBeet.serialize(unverifiedCreator);
    bytes.push(buffer);
  }
  return digest(Buffer.concat(bytes));
}

export async function leafSchemaFromLeafData(
  owner: PublicKey,
  delegate: PublicKey,
  treeId: PublicKey,
  newLeafData: NewLeafEvent
): Promise<LeafSchemaEvent> {
  const id = await getLeafAssetId(treeId, newLeafData.nonce);
  return {
    schema: {
      v1: {
        id,
        owner,
        delegate,
        dataHash: [...hashMetadata(newLeafData.metadata)],
        creatorHash: [...hashCreators(newLeafData.metadata.creators)],
        nonce: newLeafData.nonce,
      }
    }
  }
}

export function digest(input: Buffer): Buffer {
  return Buffer.from(keccak_256.digest(input))
}


function getTokenProgramVersion(object: Object): TokenProgramVersion {
  if (Object.keys(object).includes("original")) {
    return TokenProgramVersion.Original
  } else if (Object.keys(object).includes("token2022")) {
    return TokenProgramVersion.Token2022
  } else {
    return object as TokenProgramVersion;
  }
}

function getTokenStandard(object: Object): TokenStandard {
  if (!object) { return null };
  const keys = Object.keys(object);
  if (keys.includes("nonFungible")) {
    return TokenStandard.NonFungible
  } else if (keys.includes("fungible")) {
    return TokenStandard.Fungible
  } else if (keys.includes("fungibleAsset")) {
    return TokenStandard.FungibleAsset
  } else if (keys.includes("nonFungibleEdition")) {
    return TokenStandard.NonFungibleEdition
  } else {
    return object as TokenStandard;
  }
}

/// Returns number of instructions read through
export function findWrapInstructions(
  accountKeys: PublicKey[],
  instructions: CompiledInstruction[],
  amount: number,
): [CompiledInstruction[], number] {
  let count = 0;
  let found: CompiledInstruction[] = [];
  while (found.length < amount && count < instructions.length) {
    const ix = instructions[count];
    if (accountKeys[ix.programIdIndex].equals(CANDY_WRAPPER_PROGRAM_ID)) {
      found.push(ix);
    }
    count += 1;
  }
  if (found.length < amount) {
    throw new Error(`Unable to find ${amount} wrap instructions: found ${found.length}`)
  }
  return [found, count];
}
