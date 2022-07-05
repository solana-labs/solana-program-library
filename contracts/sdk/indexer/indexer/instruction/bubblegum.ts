import { hash, NFTDatabaseConnection } from "../../db"
import { ParserState, OptionalInfo } from "../utils"
import { PublicKey, CompiledInstruction, CompiledInnerInstruction } from "@solana/web3.js"
import { BorshInstructionCoder } from "@project-serum/anchor";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { ChangeLogEvent, ingestBubblegumCreateTree, ingestBubblegumMint, ingestBubblegumReplaceLeaf, LeafSchemaEvent, NewLeafEvent } from "../ingester";
import { findWrapInstructions, decodeEventInstructionData, destructureBubblegumMintAccounts, leafSchemaFromLeafData } from "../utils";

/// Copied from https://github.com/solana-labs/solana/blob/d07b0798504f757340868d15c199aba9bd00ba5d/explorer/src/utils/anchor.tsx#L57
export async function parseBubblegumInstruction(
    db: NFTDatabaseConnection,
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo,
    accountKeys: PublicKey[],
    instruction: CompiledInstruction,
    innerInstructions: CompiledInnerInstruction[],
) {
    const coder = new BorshInstructionCoder(parser.Bubblegum.idl);
    const decodedIx = coder.decode(bs58.decode(instruction.data));
    if (decodedIx) {
        const name = decodedIx.name.charAt(0).toUpperCase() + decodedIx.name.slice(1);
        console.log(`Found: ${name}`);
        switch (name) {
            case "CreateTree":
                await parseBubblegumCreateTree(
                    db,
                    slot,
                    optionalInfo,
                    parser,
                    accountKeys,
                    innerInstructions
                )
                break;
            case "MintV1":
                await parseBubblegumMint(
                    db,
                    slot,
                    optionalInfo,
                    parser,
                    accountKeys,
                    instruction,
                    innerInstructions
                )
                break;
            case "Redeem":
                await parseBubblegumReplaceLeafInstruction(
                    db,
                    slot,
                    optionalInfo,
                    parser,
                    accountKeys,
                    innerInstructions,
                    false
                );
                break
            case "Burn":
            case "CancelRedeem":
            case "Delegate":
            case "Transfer":
                await parseBubblegumReplaceLeafInstruction(
                    db,
                    slot,
                    optionalInfo,
                    parser,
                    accountKeys,
                    innerInstructions,
                )
                break;
            default:
                break
        }
    } else {
        console.error("Could not decode Bubblegum found in slot:", slot);
    }
}

async function parseBubblegumCreateTree(
    db: NFTDatabaseConnection,
    slot: number,
    optionalInfo: OptionalInfo,
    parser: ParserState,
    accountKeys: PublicKey[],
    innerInstructions: CompiledInnerInstruction[],
) {
    let changeLogEvent: ChangeLogEvent | null = null;
    for (const innerInstruction of innerInstructions) {
        const [wrapIxs] = findWrapInstructions(accountKeys, innerInstruction.instructions, 1);
        changeLogEvent = decodeEventInstructionData(parser.Gummyroll.idl, "ChangeLogEvent", wrapIxs[0].data).data as ChangeLogEvent;
    }

    await ingestBubblegumCreateTree(
        db,
        slot,
        optionalInfo,
        changeLogEvent,
    );
}



async function parseBubblegumMint(
    db: NFTDatabaseConnection,
    slot: number,
    optionalInfo: OptionalInfo,
    parser: ParserState,
    accountKeys: PublicKey[],
    instruction: CompiledInstruction,
    innerInstructions: CompiledInnerInstruction[],
) {
    let newLeafData: NewLeafEvent;
    let changeLogEvent: ChangeLogEvent;
    for (const innerInstruction of innerInstructions) {
        const [wrapIxs] = findWrapInstructions(accountKeys, innerInstruction.instructions, 2);
        newLeafData = decodeEventInstructionData(parser.Bubblegum.idl, "NewNFTEvent", wrapIxs[0].data).data as NewLeafEvent;
        changeLogEvent = decodeEventInstructionData(parser.Gummyroll.idl, "ChangeLogEvent", wrapIxs[1].data).data as ChangeLogEvent;
    }

    const { owner, delegate, merkleSlab } = destructureBubblegumMintAccounts(accountKeys, instruction);
    const leafSchema = await leafSchemaFromLeafData(owner, delegate, merkleSlab, newLeafData);

    await ingestBubblegumMint(
        db,
        slot,
        optionalInfo,
        changeLogEvent,
        newLeafData,
        leafSchema,
    )
}

async function parseBubblegumReplaceLeafInstruction(
    db: NFTDatabaseConnection,
    slot: number,
    optionalInfo: OptionalInfo,
    parser: ParserState,
    accountKeys: PublicKey[],
    innerInstructions: CompiledInnerInstruction[],
    compressed: boolean = true
) {
    let leafSchema: LeafSchemaEvent;
    let changeLogEvent: ChangeLogEvent;
    for (const innerInstruction of innerInstructions) {
        const [wrapIxs] = findWrapInstructions(accountKeys, innerInstruction.instructions, 2);
        leafSchema = decodeEventInstructionData(parser.Bubblegum.idl, "LeafSchemaEvent", wrapIxs[0].data).data as LeafSchemaEvent;
        changeLogEvent = decodeEventInstructionData(parser.Gummyroll.idl, "ChangeLogEvent", wrapIxs[1].data).data as ChangeLogEvent;
    }

    await ingestBubblegumReplaceLeaf(
        db,
        slot,
        optionalInfo,
        changeLogEvent,
        leafSchema,
        compressed
    )
}
