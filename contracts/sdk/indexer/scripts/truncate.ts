import {
    Keypair,
    Connection,
    TransactionResponse,
    TransactionInstruction,
    PublicKey,
    SYSVAR_SLOT_HASHES_PUBKEY,
    SYSVAR_INSTRUCTIONS_PUBKEY,
    LAMPORTS_PER_SOL,
    SystemProgram,
    ComputeBudgetProgram,
} from "@solana/web3.js";
import * as anchor from '@project-serum/anchor';
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { CANDY_WRAPPER_PROGRAM_ID } from "../../utils";
import { getBubblegumAuthorityPDA, getCreateTreeIxs, getLeafAssetId } from "../../bubblegum/src/convenience";
import { addProof, getMerkleRollAccountSize, PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from '../../gummyroll';
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "../../bubblegum/src/generated";
import {
    TokenStandard,
    MetadataArgs,
    TokenProgramVersion,
    createTransferInstruction,
    createMintV1Instruction,
    LeafSchema,
    leafSchemaBeet,
} from "../../bubblegum/src/generated";
import { hashCreators, hashMetadata } from "../indexer/utils";
import { BN } from "@project-serum/anchor";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import fetch from "node-fetch";
import { keccak_256 } from 'js-sha3';
import { BinaryWriter } from 'borsh';
import { createAddConfigLinesInstruction, createInitializeGumballMachineIxs, decodeGumballMachine, EncodeMethod, GumballMachine, gumballMachineHeaderBeet, InitializeGumballMachineInstructionArgs, initializeGumballMachineIndices } from "../../gumball-machine";
import { getWillyWonkaPDAKey } from "../../gumball-machine";
import { createDispenseNFTForSolIx } from "../../gumball-machine";
import { loadPrograms } from "../indexer/utils";
import { strToByteArray, execute } from "../../utils";
import { NATIVE_MINT } from "@solana/spl-token";

// const url = "http://api.explorer.mainnet-beta.solana.com";
const url = "http://127.0.0.1:8899";

function keypairFromString(seed: string) {
    const spaces = "                                         ";
    const buffer = Buffer.from(`${seed}${spaces}`.slice(0, 32));;
    return Keypair.fromSeed(Uint8Array.from(buffer));
}

const MAX_BUFFER_SIZE = 256;
const MAX_DEPTH = 20;
const CANOPY_DEPTH = 5;

/**
 * Truncates logs by sending too many append instructions
 * This forces the indexer to go into gap-filling mode
 * and use the WRAP CPI args to complete the database.
 */
async function main() {
    const endpoint = url;
    const connection = new Connection(endpoint, "confirmed");
    const payer = keypairFromString('bubblegum-mini-milady');
    const provider = new anchor.Provider(connection, new NodeWallet(payer), {
        commitment: "confirmed",
    });

    // // TODO: add gumball-machine version of truncate(test cpi indexing using instruction data)
    // let { txId, tx } = await truncateViaBubblegum(connection, provider, payer);
    // checkTxTruncated(tx);

    // // TOOD: add this after gumball-machine mints
    // let results = await testWithBubblegumTransfers(connection, provider, payer);
    // results.txs.map((tx) => {
    //     checkTxTruncated(tx);
    // })

    const { GumballMachine } = loadPrograms(provider);
    await truncateWithGumball(
        connection, provider, payer, GumballMachine
    );
}

function checkTxTruncated(tx: TransactionResponse) {
    if (tx.meta.logMessages) {
        let logsTruncated = false;
        for (const log of tx.meta.logMessages) {
            if (log.startsWith('Log truncated')) {
                logsTruncated = true;
            }
        }
        console.log(`Logs truncated: ${logsTruncated}`);
    } else {
        console.error("NO LOG MESSAGES FOUND AT ALL...error!!!")
    }
}

function getMetadata(num: number): MetadataArgs {
    return {
        name: `${num}`,
        symbol: `MILADY`,
        uri: "http://remilia.org",
        sellerFeeBasisPoints: 0,
        primarySaleHappened: false,
        isMutable: false,
        uses: null,
        collection: null,
        creators: [],
        tokenProgramVersion: TokenProgramVersion.Original,
        tokenStandard: TokenStandard.NonFungible,
        editionNonce: 0,
    }
}

async function truncateViaBubblegum(
    connection: Connection,
    provider: anchor.Provider,
    payer: Keypair,
) {
    const bgumTree = keypairFromString("bubblegum-mini-tree");
    const authority = await getBubblegumAuthorityPDA(bgumTree.publicKey);

    const acctInfo = await connection.getAccountInfo(bgumTree.publicKey, "confirmed");
    let createIxs = [];
    if (!acctInfo || acctInfo.lamports === 0) {
        console.log("Creating tree:", bgumTree.publicKey.toBase58());
        console.log("Requesting airdrop:", await connection.requestAirdrop(payer.publicKey, 5e10));
        createIxs = await getCreateTreeIxs(connection, MAX_DEPTH, MAX_BUFFER_SIZE, CANOPY_DEPTH, payer.publicKey, bgumTree.publicKey, payer.publicKey);
        console.log("<Creating tree in the truncation tx>");
    } else {
        console.log("Bubblegum tree already exists:", bgumTree.publicKey.toBase58());
    }

    const mintIxs = [];
    for (let i = 0; i < 6; i++) {
        const metadata = getMetadata(i);
        mintIxs.push(createMintV1Instruction(
            {
                owner: payer.publicKey,
                delegate: payer.publicKey,
                authority,
                candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
                gummyrollProgram: GUMMYROLL_PROGRAM_ID,
                mintAuthority: payer.publicKey,
                merkleSlab: bgumTree.publicKey,
            },
            { message: metadata }
        ));
    }
    console.log("Sending multiple mint ixs in a transaction");
    const ixs = createIxs.concat(mintIxs);
    const txId = await execute(provider, ixs, [payer, bgumTree], true);
    console.log(`Executed multiple mint ixs here: ${txId}`);
    const tx = await connection.getTransaction(txId, { commitment: 'confirmed' });
    return { txId, tx };
}

type ProofResult = {
    dataHash: number[],
    creatorHash: number[],
    root: number[],
    proofNodes: Buffer[],
    nonce: number,
    index: number,
}

async function getTransferInfoFromServer(leafHash: Buffer, treeId: PublicKey): Promise<ProofResult> {
    const proofServerUrl = "http://127.0.0.1:4000/proof";
    const hash = bs58.encode(leafHash);
    const url = `${proofServerUrl}?leafHash=${hash}&treeId=${treeId.toString()}`;
    const response = await fetch(
        url,
        { method: "GET" }
    );
    const proof = await response.json();
    return {
        dataHash: [...bs58.decode(proof.dataHash as string)],
        creatorHash: [...bs58.decode(proof.creatorHash as string)],
        root: [...bs58.decode(proof.root as string)],
        proofNodes: (proof.proofNodes as string[]).map((node) => bs58.decode(node)),
        nonce: proof.nonce,
        index: proof.index,
    };
}

// todo: expose somewhere in utils
function digest(input: Buffer): Buffer {
    return Buffer.from(keccak_256.digest(input))
}

/// Typescript impl of LeafSchema::to_node()
function hashLeafSchema(leafSchema: LeafSchema, dataHash: Buffer, creatorHash: Buffer): Buffer {
    // Fix issue with solita, the following code should work, but doesn't seem to
    // const result = leafSchemaBeet.toFixedFromValue(leafSchema);
    // const buffer = Buffer.alloc(result.byteSize);
    // result.write(buffer, 0, leafSchema);

    const writer = new BinaryWriter();
    // When we have versions other than V1, we definitely want to use solita
    writer.writeU8(1);
    writer.writeFixedArray(leafSchema.id.toBuffer());
    writer.writeFixedArray(leafSchema.owner.toBuffer());
    writer.writeFixedArray(leafSchema.delegate.toBuffer());
    writer.writeFixedArray(new BN(leafSchema.nonce).toBuffer('le', 8));
    writer.writeFixedArray(dataHash);
    writer.writeFixedArray(creatorHash);
    const buf = Buffer.from(writer.toArray());
    return digest(buf);
}

async function testWithBubblegumTransfers(
    connection: Connection,
    provider: anchor.Provider,
    payer: Keypair,
) {
    const bgumTree = keypairFromString("bubblegum-mini-tree");
    const authority = await getBubblegumAuthorityPDA(bgumTree.publicKey);

    // const acctInfo = await connection.getAccountInfo(bgumTree.publicKey, "confirmed");
    // const merkleRoll = decodeMerkleRoll(acctInfo.data);
    // const root = Array.from(merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBytes());

    const txIds = [];
    const txs = [];
    const finalDestination = keypairFromString("bubblegum-final-destination");
    for (let i = 0; i < 6; i++) {
        const metadata = getMetadata(i);
        const computedDataHash = hashMetadata(metadata);
        const computedCreatorHash = hashCreators(metadata.creators);
        const leafSchema: LeafSchema = {
            __kind: "V1",
            id: await getLeafAssetId(bgumTree.publicKey, new BN(i)),
            owner: payer.publicKey,
            delegate: payer.publicKey,
            nonce: new BN(i),
            dataHash: [...computedDataHash],
            creatorHash: [...computedCreatorHash],
        };
        const leafHash = hashLeafSchema(leafSchema, computedDataHash, computedCreatorHash);
        console.log("Data hash:", bs58.encode(computedDataHash));
        console.log("Creator hash:", bs58.encode(computedCreatorHash));
        console.log("schema:", {
            id: leafSchema.id.toString(),
            owner: leafSchema.owner.toString(),
            delegate: leafSchema.owner.toString(),
            nonce: new BN(i),
        });
        const { root, dataHash, creatorHash, proofNodes, nonce, index } = await getTransferInfoFromServer(leafHash, bgumTree.publicKey);
        const transferIx = addProof(createTransferInstruction({
            authority,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
            gummyrollProgram: GUMMYROLL_PROGRAM_ID,
            owner: payer.publicKey,
            delegate: payer.publicKey,
            newOwner: finalDestination.publicKey,
            merkleSlab: bgumTree.publicKey,
        }, {
            dataHash,
            creatorHash,
            nonce,
            root,
            index,
        }), proofNodes.slice(0, MAX_DEPTH - CANOPY_DEPTH));
        txIds.push(await execute(provider, [transferIx], [payer], true));
        txs.push(await connection.getTransaction(txIds[txIds.length - 1], { commitment: 'confirmed' }));
    }
    console.log(`Transferred all NFTs to ${finalDestination.publicKey.toString()}`);
    console.log(`Executed multiple transfer ixs here: ${txIds}`);
    return { txIds, txs };
}

async function initializeGumballMachine(
    payer: Keypair,
    authority: Keypair,
    gumballMachineAcctKeypair: Keypair,
    gumballMachineAcctSize: number,
    merkleRollKeypair: Keypair,
    merkleRollAccountSize: number,
    gumballMachineInitArgs: InitializeGumballMachineInstructionArgs,
    mint: PublicKey,
    gumballMachine: anchor.Program<GumballMachine>,
) {
    const initializeGumballMachineInstrs =
        await createInitializeGumballMachineIxs(
            payer.publicKey,
            gumballMachineAcctKeypair.publicKey,
            gumballMachineAcctSize,
            merkleRollKeypair.publicKey,
            merkleRollAccountSize,
            gumballMachineInitArgs,
            mint,
            gumballMachine.provider.connection
        );
    console.log(`${payer.publicKey.toString()}`);
    await execute(
        gumballMachine.provider,
        initializeGumballMachineInstrs,
        [payer, gumballMachineAcctKeypair, merkleRollKeypair],
        true
    );
    await initializeGumballMachineIndices(gumballMachine.provider, gumballMachineInitArgs.maxItems, authority, gumballMachineAcctKeypair.publicKey);
}

async function addConfigLines(
    provider: anchor.Provider,
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    configLinesToAdd: Uint8Array,
) {
    const addConfigLinesInstr = createAddConfigLinesInstruction(
        {
            gumballMachine: gumballMachineAcctKey,
            authority: authority.publicKey,
        },
        {
            newConfigLinesData: configLinesToAdd,
        }
    );
    await execute(
        provider,
        [addConfigLinesInstr],
        [authority]
    )
}

async function dispenseCompressedNFTForSol(
    numNFTs: number,
    payer: Keypair,
    receiver: PublicKey,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    gumballMachine: anchor.Program<GumballMachine>,
) {
    const additionalComputeBudgetInstruction = ComputeBudgetProgram.requestUnits({
        units: 1000000,
        additionalFee: 0,
    });
    const dispenseInstr = await createDispenseNFTForSolIx(
        { numItems: numNFTs },
        payer.publicKey,
        receiver,
        gumballMachineAcctKeypair.publicKey,
        merkleRollKeypair.publicKey
    );
    const txId = await execute(
        gumballMachine.provider,
        [additionalComputeBudgetInstruction, dispenseInstr],
        [payer],
        true
    );
}

async function truncateWithGumball(
    connection: Connection,
    provider: anchor.Provider,
    payer: Keypair,
    gumballMachine: anchor.Program<GumballMachine>,
) {
    const EXTENSION_LEN = 28;
    const MAX_MINT_SIZE = 10;
    const GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE = 1000;
    const GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE = 7000;
    const GUMBALL_MACHINE_ACCT_SIZE =
        gumballMachineHeaderBeet.byteSize +
        GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE +
        GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE;
    const MERKLE_ROLL_ACCT_SIZE = getMerkleRollAccountSize(3, 8);

    const creatorAddress = keypairFromString('gumball-machine-creat0r');
    const gumballMachineAcctKeypair = keypairFromString('gumball-machine-acct')
    const merkleRollKeypair = keypairFromString("gumball-machine-tree");
    const nftBuyer = keypairFromString("gumball-machine-buyer")
    const botWallet = Keypair.generate();

    // Give creator enough funds to produce accounts for gumball-machine
    await connection.requestAirdrop(
        creatorAddress.publicKey,
        4 * LAMPORTS_PER_SOL,
    );
    await connection.requestAirdrop(
        payer.publicKey,
        11 * LAMPORTS_PER_SOL,
    );
    await connection.requestAirdrop(
        nftBuyer.publicKey,
        11 * LAMPORTS_PER_SOL,
    );
    console.log("airdrop successfull")

    const baseGumballMachineInitProps: InitializeGumballMachineInstructionArgs = {
        maxDepth: 3,
        maxBufferSize: 8,
        urlBase: strToByteArray("https://arweave.net", 64),
        nameBase: strToByteArray("Milady", 32),
        symbol: strToByteArray("MILADY", 8),
        sellerFeeBasisPoints: 100,
        isMutable: true,
        retainAuthority: true,
        encodeMethod: EncodeMethod.Base58Encode,
        price: new BN(0.1),
        goLiveDate: new BN(1234.0),
        botWallet: botWallet.publicKey,
        receiver: creatorAddress.publicKey,
        authority: creatorAddress.publicKey,
        collectionKey: SystemProgram.programId, // 0x0 -> no collection key
        extensionLen: new BN(EXTENSION_LEN),
        maxMintSize: MAX_MINT_SIZE,
        maxItems: 250,
        creatorKeys: [creatorAddress.publicKey],
        creatorShares: Uint8Array.from([100]),
    };

    if (!(await connection.getAccountInfo(merkleRollKeypair.publicKey, "confirmed"))) {
        await initializeGumballMachine(
            creatorAddress,
            creatorAddress,
            gumballMachineAcctKeypair,
            GUMBALL_MACHINE_ACCT_SIZE,
            merkleRollKeypair,
            MERKLE_ROLL_ACCT_SIZE,
            baseGumballMachineInitProps,
            NATIVE_MINT,
            gumballMachine
        );
        console.log('init`d');
    }

    // add 10 config lines
    let arr: number[] = [];
    const buffers = [];
    for (let i = 0; i < 6; i++) {
        const str = `url-${i}                                         `.slice(0, EXTENSION_LEN);
        arr = arr.concat(strToByteArray(str));
        buffers.push(Buffer.from(str));
    }
    await addConfigLines(
        provider,
        creatorAddress,
        gumballMachineAcctKeypair.publicKey,
        Buffer.from(arr),
    );

    await dispenseCompressedNFTForSol(
        6,
        nftBuyer,
        creatorAddress.publicKey,
        gumballMachineAcctKeypair,
        merkleRollKeypair,
        gumballMachine
    )
}

main();
