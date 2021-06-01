/// <reference types="node" />
import { Connection, PublicKey, TransactionInstruction } from "@solana/web3.js";
import { SerializationMethod } from "../idl/idl-coder";
export declare const NS_HASH_PREFIX = "SPL Name Service";
export declare const NS_PROGRAM_ID: PublicKey;
export declare const METADATA_PREFIX = "program_metadata";
export declare const PROGRAM_METADATA_ID: PublicKey;
export interface ProgramMetadataConfig {
    programMetadataKey?: PublicKey;
    nameServiceKey?: PublicKey;
}
export declare class ProgramMetadata {
    private connection;
    programMetadataKey: PublicKey;
    nameServiceKey: PublicKey;
    constructor(connection: Connection, config?: ProgramMetadataConfig);
    createMetadataEntry(targetProgramId: PublicKey, targetProgramAuthorityKey: PublicKey, payerKey: PublicKey, name: string, value: string): Promise<TransactionInstruction>;
    updateMetadataEntry(targetProgramId: PublicKey, targetProgramAuthorityKey: PublicKey, name: string, value: string): Promise<TransactionInstruction>;
    deleteMetadataEntry(targetProgramId: PublicKey, targetProgramAuthorityKey: PublicKey, refundKey: PublicKey, name: string): Promise<TransactionInstruction>;
    createVersionedIdl(targetProgramId: PublicKey, targetProgramAuthorityKey: PublicKey, payerKey: PublicKey, effectiveSlot: number, idlUrl: string, idlHash: Buffer, sourceUrl: string, serializiation: SerializationMethod, customLayoutUrl: string | null): Promise<TransactionInstruction>;
    updateVersionedIdl(targetProgramId: PublicKey, targetProgramAuthorityKey: PublicKey, effectiveSlot: number, idlUrl: string, idlHash: Buffer, sourceUrl: string, serialization: SerializationMethod, customLayoutUrl: string | null): Promise<TransactionInstruction>;
    getHashedName(name: any): Buffer;
    getClassKey(targetProgramId: PublicKey): Promise<PublicKey>;
    getNameKey(hashedName: Buffer, classKey: PublicKey): Promise<PublicKey>;
}
