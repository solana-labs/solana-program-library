import {
    decodeMerkleRoll,
    getMerkleRollAccountSize,
} from "../accounts";
import {
    PublicKey,
    Keypair,
    SystemProgram,
    Transaction,
    Connection as web3Connection,
    LAMPORTS_PER_SOL,
    Connection,
  } from "@solana/web3.js";

export async function getRootOfOnChainMerkleRoot(connection: Connection, merkleRollAccountKey: PublicKey): Promise<Buffer> {
    const merkleRootAcct = await connection.getAccountInfo(merkleRollAccountKey);
    if (!merkleRootAcct) {
        throw new Error("Merkle Root account data unexpectedly null!");
    }
    const merkleRoll = decodeMerkleRoll(merkleRootAcct.data);
    return merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();
}

