import { NFTDatabaseConnection } from "../../db"
import {
    ParserState,
    OptionalInfo,
    decodeEventInstructionData,
    leafSchemaFromLeafData,
    destructureBubblegumMintAccounts,
    findWrapInstructions
} from "../utils"
import { PublicKey, CompiledInstruction } from "@solana/web3.js"
import { BorshInstructionCoder } from "@project-serum/anchor";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { ChangeLogEvent, ingestBubblegumCreateTree, ingestBubblegumMint, ingestBubblegumReplaceLeaf, LeafSchemaEvent, NewLeafEvent } from "../ingester";

/**
 *  This kind of difficult because there is no depth associated with the inner instructions
 */
export async function parseBubblegumInnerInstructions(
    db: NFTDatabaseConnection,
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo,
    accountKeys: PublicKey[],
    innerInstructions: CompiledInstruction[],
) {
    let i = 0;
    while (i < innerInstructions.length) {
        const programId = accountKeys[innerInstructions[i].programIdIndex];
        if (programId.equals(parser.Bubblegum.programId)) {
            i = await parseBubblegumExecutionContext(db, slot, parser, optionalInfo, accountKeys, innerInstructions, i);
        }
        i++;
    }
}


async function parseBubblegumCreateTreeInstructions(
    db: NFTDatabaseConnection,
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo,
    accountKeys: PublicKey[],
    instructions: CompiledInstruction[],
    currentIndex: number
): Promise<number> {
    const [found, count] = findWrapInstructions(
        accountKeys,
        instructions.slice(currentIndex),
        1
    );
    const changeLogEvent = decodeEventInstructionData(
        parser.Gummyroll.idl,
        "ChangeLogEvent",
        found[0].data
    ).data as ChangeLogEvent;
    await ingestBubblegumCreateTree(
        db,
        slot,
        optionalInfo,
        changeLogEvent
    );
    return currentIndex + count;
}

async function parseBubblegumMintInstructions(
    db: NFTDatabaseConnection,
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo,
    accountKeys: PublicKey[],
    instructions: CompiledInstruction[],
    currentIndex: number
): Promise<number> {
    const [found, count] = findWrapInstructions(
        accountKeys,
        instructions.slice(currentIndex + 1),
        2
    );
    const newLeafData = decodeEventInstructionData(
        parser.Bubblegum.idl,
        "NewNFTEvent",
        found[0].data
    ).data as NewLeafEvent;
    const changeLogEvent = decodeEventInstructionData(
        parser.Gummyroll.idl,
        "ChangeLogEvent",
        found[1].data
    ).data as ChangeLogEvent;

    const { owner, delegate, merkleSlab } = destructureBubblegumMintAccounts(
        accountKeys,
        instructions[currentIndex]
    );
    const leafSchema = await leafSchemaFromLeafData(owner, delegate, merkleSlab, newLeafData);

    await ingestBubblegumMint(
        db,
        slot,
        optionalInfo,
        changeLogEvent,
        newLeafData,
        leafSchema
    );
    return currentIndex + count
}

/// Untested
/// Todo: test
async function parseBubblegumReplaceLeafInstructions(
    db: NFTDatabaseConnection,
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo,
    accountKeys: PublicKey[],
    instructions: CompiledInstruction[],
    currentIndex: number,
    compressed: boolean = true
): Promise<number> {
    const [found, count] = findWrapInstructions(
        accountKeys,
        instructions,
        2
    );
    const leafSchema = decodeEventInstructionData(
        parser.Bubblegum.idl,
        "LeafSchemaEvent",
        found[0].data
    ).data as LeafSchemaEvent;
    const changeLogEvent = decodeEventInstructionData(
        parser.Gummyroll.idl,
        "ChangeLogEvent",
        found[1].data
    ).data as ChangeLogEvent;
    await ingestBubblegumReplaceLeaf(
        db,
        slot,
        optionalInfo,
        changeLogEvent,
        leafSchema,
        compressed
    )
    return currentIndex + count
}

/**
 * Here we know that instructions at current index may actually CPIs from bubblegum
 */
async function parseBubblegumExecutionContext(
    db: NFTDatabaseConnection,
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo,
    accountKeys: PublicKey[],
    instructions: CompiledInstruction[],
    currentIndex: number
): Promise<number> {
    const coder = new BorshInstructionCoder(parser.Bubblegum.idl);
    const instruction = instructions[currentIndex];
    const decodedIx = coder.decode(bs58.decode(instruction.data));
    if (decodedIx) {
        const name = decodedIx.name.charAt(0).toUpperCase() + decodedIx.name.slice(1);
        console.log(`Found: ${name}`);
        switch (name) {
            case "CreateTree":
                return await parseBubblegumCreateTreeInstructions(
                    db,
                    slot,
                    parser,
                    optionalInfo,
                    accountKeys,
                    instructions,
                    currentIndex
                );
            case "MintV1":
                return await parseBubblegumMintInstructions(
                    db,
                    slot,
                    parser,
                    optionalInfo,
                    accountKeys,
                    instructions,
                    currentIndex
                );
            /// TODO(ngundotra): add tests for the following leaf-replacements
            case "Redeem":
                return await parseBubblegumReplaceLeafInstructions(
                    db,
                    slot,
                    parser,
                    optionalInfo,
                    accountKeys,
                    instructions,
                    currentIndex,
                    false
                )
            case "Burn":
            case "CancelRedeem":
            case "Delegate":
            case "Transfer":
                return await parseBubblegumReplaceLeafInstructions(
                    db,
                    slot,
                    parser,
                    optionalInfo,
                    accountKeys,
                    instructions,
                    currentIndex
                )
            default:
                break
        }
    }
    return currentIndex
}

