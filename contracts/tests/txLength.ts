import {
    PublicKey,
    Keypair,
    Transaction,
    Connection,
} from '@solana/web3.js';
import { buildTree, emptyNode, Tree, } from './merkle-tree';
import { Program, Idl, Provider } from '@project-serum/anchor';
import { Gummyroll } from "../target/types/gummyroll";
import { IDL } from '../target/types/gummyroll';
import { IDL as CrudIDL } from '../target/types/gummyroll_crud';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { GummyrollCrud } from '../target/types/gummyroll_crud';
import * as crypto from 'crypto';

const PROGRAM_ID = "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD";

const payer = Keypair.generate();
const payerWallet = new NodeWallet(payer);
const connection = new Connection("http://localhost:8899");
const provider = new Provider(connection,
    payerWallet,
    { skipPreflight: true, commitment: "confirmed" }
);
const gummyroll = new Program<Gummyroll>(
    IDL,
    new PublicKey(PROGRAM_ID),
    provider,
);

const gummyrollCrud = new Program<GummyrollCrud>(
    CrudIDL,
    new PublicKey(PROGRAM_ID),
    provider
);

/**
 * This is the largest such instruction that can be made with the raw gummyroll program
 * We want to cut this down as much as possible
 */
function createReplaceIx(
    root: Buffer,
    oldLeaf: Buffer,
    newLeaf: Buffer,
    merkleRollKeypair: Keypair,
    payer: Keypair,
    i: number,
    nodes: Node[],
) {
    const nodeProof = nodes.map((node) => {
        return {
            pubkey: new PublicKey(node),
            isSigner: false,
            isWritable: false
        }
    });

    const replaceLeafIx = gummyroll.instruction.replaceLeaf(
        { inner: Array.from(root) },
        { inner: Array.from(oldLeaf) },
        { inner: Array.from(newLeaf) },
        i,
        {
            accounts: {
                merkleRoll: merkleRollKeypair.publicKey,
                authority: payer.publicKey,
            },
            signers: [payer],
            remainingAccounts: nodeProof,
        }
    );
    return replaceLeafIx;
}

async function getMaxTxSize(maxDepth: number): Promise<number> {
    let nodes = [];
    for (let i = 0; i <= maxDepth; i++) {
        nodes.push(emptyNode(i));
    }

    const merkleRollKeypair = Keypair.generate();
    const replaceIx = createReplaceIx(
        emptyNode(maxDepth),
        Buffer.alloc(32),
        Buffer.alloc(32, 1),
        merkleRollKeypair,
        payer,
        0,
        nodes,
    );

    let tx = new Transaction().add(replaceIx);
    const merkleWallet = new NodeWallet(merkleRollKeypair);
    tx.feePayer = payer.publicKey;
    tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
    tx.sign(payer);

    let length: number;
    try {
        const serialized = tx.serialize();
        length = serialized.length;
    } catch (e) {
        length = -1;
    }
    return length
}

async function getMaxCrudTxSize(maxDepth: number): Promise<number> {
    const treeAdminKeypair = Keypair.generate();
    const ownerKeypair = Keypair.generate();
    const signers = [ownerKeypair];

    let nodes = [];
    for (let i = 0; i <= maxDepth; i++) {
        nodes.push(emptyNode(i));
    }
    const proofPubkeys = nodes.map((node) => {
        return {
            pubkey: new PublicKey(node),
            isSigner: false,
            isWritable: false,
        }
    });

    const transferIx = gummyrollCrud.instruction.transfer(
        Buffer.alloc(32),
        Buffer.alloc(32),
        0,
        {
            accounts: {
                authority: treeAdminKeypair.publicKey,
                authorityPda: Keypair.generate().publicKey,
                gummyrollProgram: gummyroll.programId,
                merkleRoll: Keypair.generate().publicKey,
                newOwner: Keypair.generate().publicKey,
                owner: ownerKeypair.publicKey,
            },
            signers,
            remainingAccounts: proofPubkeys,
        }
    );
    const tx = new Transaction().add(transferIx);

    tx.feePayer = payer.publicKey;
    tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
    tx.sign(payer, ownerKeypair);

    let length: number;
    try {
        const serialized = tx.serialize();
        length = serialized.length;
    } catch (e) {
        length = -1;
    }
    return length
}

async function getGummyrollMaxAppendTxSize(numAppends: number): Promise<number> {
    const merkleRollKeypair = Keypair.generate();

    let tx = new Transaction()
    for (let i = 0; i < numAppends; i++) {
        tx = tx.add(
            gummyroll.instruction.append(
                {
                    inner: Array.from(crypto.randomBytes(32))
                },
                {
                    accounts: {
                        merkleRoll: merkleRollKeypair.publicKey,
                        authority: payer.publicKey,
                        appendAuthority: payer.publicKey,
                    },
                    signers: [payer],
                }
            ));
    }

    tx.feePayer = payer.publicKey;
    tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
    tx.sign(payer);

    let length: number;
    try {
        const serialized = tx.serialize();
        length = serialized.length;
    } catch (e) {
        length = -1;
    }
    return length
}

async function getGummyrollCrudMaxAppendTxSize(numAppends: number): Promise<number> {
    const treeAdminKeypair = Keypair.generate();
    const owner = Keypair.generate();
    const merkleRollKeypair = Keypair.generate();
    const authorityPda = Keypair.generate().publicKey;

    let tx = new Transaction()
    for (let i = 0; i < numAppends; i++) {
        tx = tx.add(
            gummyrollCrud.instruction.add(crypto.randomBytes(32), {
                accounts: {
                    authority: treeAdminKeypair.publicKey,
                    authorityPda,
                    gummyrollProgram: gummyroll.programId,
                    merkleRoll: merkleRollKeypair.publicKey,
                },
                signers: [treeAdminKeypair],
            }));
    }

    // tx.feePayer = payer.publicKey;
    tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
    tx.sign(treeAdminKeypair);

    let length: number;
    try {
        const serialized = tx.serialize();
        length = serialized.length;
    } catch (e) {
        length = -1;
    }
    return length
}

async function main() {
    console.log("| Max Depth | Max Leaves | Gummyroll Tx Size | Crud Tx Size |")
    for (let maxDepth = 14; maxDepth < 32; maxDepth++) {
        const rawSize = await getMaxTxSize(maxDepth);
        const crudSize = await getMaxCrudTxSize(maxDepth)
        console.log(
            `|${maxDepth}|${(2 ** maxDepth).toLocaleString("en-us").padStart(11)}|${rawSize.toString().padStart(4)}|${crudSize.toString().padStart(4)}|`
        );
        if (rawSize < 0 && crudSize < 0) {
            console.log("No greater max depth is possible; Exiting")
            break;
        }
    }

    // Spec out how many append tx's can fit inside a single tx
    console.log(
        `| Num Appends | Gummyroll Tx Size | Crud Tx Size |`
    );
    for (let numAppends = 1; numAppends < 25; numAppends++) {
        const rawSize = await getGummyrollMaxAppendTxSize(numAppends);
        const crudSize = await getGummyrollCrudMaxAppendTxSize(numAppends);
        console.log(
            `|${numAppends}|${rawSize.toString().padStart(4)}|${crudSize.toString().padStart(4)}|`
        );
        if (rawSize < 0 && crudSize < 0) {
            console.log("No greater number of appends is possible; Exiting")
            break;
        }
    }
}
main()
