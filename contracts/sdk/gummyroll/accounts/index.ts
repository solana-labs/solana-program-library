import {
    PublicKey
} from '@solana/web3.js';
import * as borsh from 'borsh';
import { BN } from '@project-serum/anchor';

/**
 * Manually create a model for MerkleRoll in order to deserialize correctly
 */
export type OnChainMerkleRoll = {
    header: MerkleRollHeader,
    roll: MerkleRoll
}

type MerkleRollHeader = {
    maxDepth: number, // u32
    maxBufferSize: number, // u32
    authority: PublicKey,
    appendAuthority: PublicKey,
}

type MerkleRoll = {
    sequenceNumber: BN, // u64
    activeIndex: number, // u64
    bufferSize: number, // u64
    changeLogs: ChangeLog[],
    rightMostPath: Path,
}

type ChangeLog = {
    root: PublicKey,
    pathNodes: PublicKey[]
    index: number, // u32
    _padding: number, // u32
}

type Path = {
    leaf: PublicKey,
    proof: PublicKey[],
    index: number,
    _padding: number,
};

function readPublicKey(reader: borsh.BinaryReader): PublicKey {
    return new PublicKey(reader.readFixedArray(32));
}

export function decodeMerkleRoll(buffer: Buffer): OnChainMerkleRoll {
    let reader = new borsh.BinaryReader(buffer);

    let header: MerkleRollHeader = {
        maxBufferSize: reader.readU32(),
        maxDepth: reader.readU32(),
        authority: readPublicKey(reader),
        appendAuthority: readPublicKey(reader)
    };

    // Decode MerkleRoll
    let sequenceNumber = reader.readU128();
    let activeIndex = reader.readU64().toNumber();
    let bufferSize = reader.readU64().toNumber();

    // Decode ChangeLogs
    let changeLogs = [];
    for (let i = 0; i < header.maxBufferSize; i++) {
        let root = readPublicKey(reader);

        let pathNodes = [];
        for (let j = 0; j < header.maxDepth; j++) {
            pathNodes.push(readPublicKey(reader));
        }
        changeLogs.push({
            pathNodes,
            root,
            index: reader.readU32(),
            _padding: reader.readU32(),
        });
    }

    // Decode Right-Most Path
    let leaf = readPublicKey(reader);
    let proof = [];
    for (let j = 0; j < header.maxDepth; j++) {
        proof.push(readPublicKey(reader));
    }
    const rightMostPath = {
        proof,
        leaf,
        index: reader.readU32(),
        _padding: reader.readU32(),
    }

    const roll = {
        sequenceNumber,
        activeIndex,
        bufferSize,
        changeLogs,
        rightMostPath
    }

    if (getMerkleRollAccountSize(header.maxDepth, header.maxBufferSize) != reader.offset) {
        throw new Error("Failed to process whole buffer when deserializing Merkle Account Data")
    }
    return {
        header,
        roll
    }
}

export function getMerkleRollAccountSize(maxDepth: number, maxBufferSize: number): number {
    let headerSize = 8 + 32 + 32;
    let changeLogSize = (maxDepth * 32 + 32 + 4 + 4) * maxBufferSize;
    let rightMostPathSize = maxDepth * 32 + 32 + 4 + 4;
    let merkleRollSize = 8 + 8 + 16 + changeLogSize + rightMostPathSize;
    return merkleRollSize + headerSize;
}
