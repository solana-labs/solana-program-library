import * as anchor from "@project-serum/anchor";
import {
    Keypair,
    Transaction,
    SystemProgram,
    PublicKey, Connection as web3Connection,
} from "@solana/web3.js";
import { assert, expect } from "chai";
import { GummyrollCrud } from "../target/types/gummyroll_crud";
import { Program, Provider } from "@project-serum/anchor";
import {
    Gummyroll,
    decodeMerkleRoll,
    getMerkleRollAccountSize,
} from "../sdk/gummyroll";
import { buildTree, getProofOfLeaf, hash, updateTree } from "./merkle-tree";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";

// @ts-ignore
let Gummyroll;
// @ts-ignore
let GummyrollCrud;

describe("Gummyroll CRUD program", () => {

    const MAX_DEPTH = 14;
    const MAX_BUFFER_SIZE = 64;
    const requiredSpace = getMerkleRollAccountSize(MAX_DEPTH, MAX_BUFFER_SIZE);

    let payer = Keypair.generate();
    let wallet = new NodeWallet(payer)
    let connection = new web3Connection(
        "http://localhost:8899",
        {
            commitment: 'confirmed'
        }
    );
    anchor.setProvider(new Provider(connection, wallet, { commitment: connection.commitment, skipPreflight: true }));

    Gummyroll = anchor.workspace.Gummyroll as Program<Gummyroll>;
    GummyrollCrud = anchor.workspace.GummyrollCrud as Program<GummyrollCrud>;
    let tree: ReturnType<typeof buildTree>;

    async function appendAsset(
        treeAddress: PublicKey,
        treeAdminKeypair: Keypair,
        message: string,
        config?: { overrides: { signer?: Keypair } }
    ) {
        const [treeAuthorityPDA] = await getTreeAuthorityPDA(
            treeAddress,
            treeAdminKeypair.publicKey
        );
        const signers = [config?.overrides.signer ?? treeAdminKeypair];
        const addIx = GummyrollCrud.instruction.add(Buffer.from(message), {
            accounts: {
                authority: treeAdminKeypair.publicKey,
                authorityPda: treeAuthorityPDA,
                gummyrollProgram: Gummyroll.programId,
                merkleRoll: treeAddress,
            },
            signers,
        });
        await GummyrollCrud.provider.send(new Transaction().add(addIx), signers, {
            commitment: "confirmed",
        });
    }

    async function createTree(
        treeAdminKeypair: Keypair,
        maxDepth: number,
        maxBufferSize: number
    ): Promise<[Keypair, PublicKey]> {
        const treeKeypair = Keypair.generate();
        const allocGummyrollAccountIx = SystemProgram.createAccount({
            fromPubkey: treeAdminKeypair.publicKey,
            newAccountPubkey: treeKeypair.publicKey,
            lamports:
                await Gummyroll.provider.connection.getMinimumBalanceForRentExemption(
                    requiredSpace
                ),
            space: requiredSpace,
            programId: Gummyroll.programId,
        });
        const [treeAuthorityPDA] = await getTreeAuthorityPDA(
            treeKeypair.publicKey,
            treeAdminKeypair.publicKey
        );
        const createTreeTx = GummyrollCrud.instruction.createTree(
            maxDepth,
            maxBufferSize,
            {
                accounts: {
                    authority: treeAdminKeypair.publicKey,
                    authorityPda: treeAuthorityPDA,
                    gummyrollProgram: Gummyroll.programId,
                    merkleRoll: treeKeypair.publicKey,
                },
                signers: [treeAdminKeypair],
            }
        );
        const tx = new Transaction().add(allocGummyrollAccountIx).add(createTreeTx);
        const createTreeTxId = await Gummyroll.provider.send(
            tx,
            [treeAdminKeypair, treeKeypair],
            {
                commitment: "confirmed",
            }
        );
        assert(createTreeTxId, "Failed to initialize an empty Gummyroll");
        return [treeKeypair, treeAuthorityPDA];
    }

    async function getActualRoot(treeAddress: PublicKey) {
        const treeAccount = await Gummyroll.provider.connection.getAccountInfo(
            treeAddress
        );
        const tree = decodeMerkleRoll(treeAccount.data);
        return tree.roll.changeLogs[tree.roll.activeIndex].root.toBuffer();
    }

    async function getTreeAuthorityPDA(
        treeAddress: PublicKey,
        treeAdmin: PublicKey
    ) {
        const seeds = [
            Buffer.from("gummyroll-crud-authority-pda", "utf-8"),
            treeAddress.toBuffer(),
            treeAdmin.toBuffer(),
        ];
        return await anchor.web3.PublicKey.findProgramAddress(
            seeds,
            GummyrollCrud.programId
        );
    }

    function recomputeRootByAddingLeafToTreeWithMessageAtIndex(
        owner: PublicKey,
        message: string,
        index: number
    ) {
        const newLeaf = hash(owner.toBuffer(), Buffer.from(message));
        updateTree(tree, newLeaf, index);
        return tree.root;
    }

    function recomputeRootByRemovingLeafFromTreeAtIndex(index: number) {
        const newLeaf = Buffer.alloc(32, 0);
        updateTree(tree, newLeaf, index);
        return tree.root;
    }

    let treeAdminKeypair: Keypair;
    beforeEach(async () => {
        const leaves = Array(2 ** MAX_DEPTH).fill(Buffer.alloc(32));
        tree = buildTree(leaves);
        await Gummyroll.provider.connection.confirmTransaction(
            await Gummyroll.provider.connection.requestAirdrop(payer.publicKey, 1e10),
            "confirmed"
        );
        treeAdminKeypair = Keypair.generate();
        await Gummyroll.provider.connection.confirmTransaction(
            await Gummyroll.provider.connection.requestAirdrop(
                treeAdminKeypair.publicKey,
                2e9
            ),
            "confirmed"
        );
    });

    describe("`CreateTree` instruction", () => {
        let treeKeypair: Keypair;
        let treeAuthorityPDA: PublicKey;
        beforeEach(async () => {
            const [computedTreeKeypair, computedTreeAuthorityPDA] = await createTree(
                treeAdminKeypair,
                MAX_DEPTH,
                MAX_BUFFER_SIZE
            );
            treeKeypair = computedTreeKeypair;
            treeAuthorityPDA = computedTreeAuthorityPDA;
        });
        it("creates a Merkle roll using the supplied inputs", async () => {
            const merkleRollAccount =
                await GummyrollCrud.provider.connection.getAccountInfo(
                    treeKeypair.publicKey,
                    "confirmed"
                );
            expect(merkleRollAccount).not.to.be.null;
            expect(
                merkleRollAccount.owner.equals(Gummyroll.programId),
                "Expected the tree to be owned by the Gummyroll program " +
                `\`${Gummyroll.programId.toBase58()}\`. Was owned by ` +
                `\`${merkleRollAccount.owner.toBase58()}\``
            ).to.be.true;
            expect(merkleRollAccount.data.byteLength).to.equal(requiredSpace);
            const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
            expect(merkleRoll.header.maxDepth).to.equal(MAX_DEPTH);
            expect(merkleRoll.header.maxBufferSize).to.equal(MAX_BUFFER_SIZE);
            expect(
                merkleRoll.header.authority.equals(treeAuthorityPDA),
                "Expected the tree authority to be the authority PDA " +
                `\`${treeAuthorityPDA.toBase58()}\`. Got ` +
                `\`${merkleRoll.header.authority.toBase58()}\``
            ).to.be.true;
        });
    });

    describe("`Add` instruction", () => {
        let treeKeypair: Keypair;
        beforeEach(async () => {
            const [computedTreeKeypair] = await createTree(
                treeAdminKeypair,
                MAX_DEPTH,
                MAX_BUFFER_SIZE
            );
            treeKeypair = computedTreeKeypair;
        });
        it("fails if someone other than the tree authority attempts to add an item", async () => {
            const attackerKeypair = Keypair.generate();
            try {
                await appendAsset(treeKeypair.publicKey, treeAdminKeypair, "Fake NFT", {
                    overrides: { signer: attackerKeypair },
                });
                assert(
                    false,
                    "Nobody other than the tree admin should be able to add an asset to the tree"
                );
            } catch {
            }
        });
        describe("having appended the first item", () => {
            const firstTestMessage = "First test message";
            beforeEach(async () => {
                await appendAsset(
                    treeKeypair.publicKey,
                    treeAdminKeypair,
                    firstTestMessage
                );
            });
            it("updates the root hash correctly", async () => {
                const actualRoot = await getActualRoot(treeKeypair.publicKey);
                const expectedRoot = recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                    treeAdminKeypair.publicKey,
                    firstTestMessage,
                    0
                );
                expect(expectedRoot.compare(actualRoot)).to.equal(
                    0,
                    "On-chain root hash does not equal expected hash"
                );
            });
            describe("having appended the second item", () => {
                const secondTestMessage = "Second test message";
                beforeEach(async () => {
                    await appendAsset(
                        treeKeypair.publicKey,
                        treeAdminKeypair,
                        secondTestMessage
                    );
                });
                it("updates the root hash correctly", async () => {
                    const actualRoot = await getActualRoot(treeKeypair.publicKey);
                    recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                        treeAdminKeypair.publicKey,
                        firstTestMessage,
                        0
                    );
                    const expectedRoot =
                        recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                            treeAdminKeypair.publicKey,
                            secondTestMessage,
                            1
                        );
                    expect(expectedRoot.compare(actualRoot)).to.equal(
                        0,
                        "On-chain root hash does not equal expected hash"
                    );
                });
            });
        });
    });

    describe("`Transfer` instruction", () => {
        const message = "Message";

        async function transferAsset(
            treeAddress: PublicKey,
            treeAdmin: PublicKey,
            ownerKeypair: Keypair,
            newOwnerPubkey: PublicKey,
            index: number,
            config: { overrides?: { message?: string; signer?: Keypair } } = {}
        ) {
            const [treeAuthorityPDA] = await getTreeAuthorityPDA(
                treeAddress,
                treeAdmin
            );
            const proofPubkeys = getProofOfLeaf(tree, index).map(({ node }) => ({
                pubkey: new PublicKey(node),
                isSigner: false,
                isWritable: false,
            }));
            const signers = [config.overrides?.signer ?? ownerKeypair];
            const transferIx = GummyrollCrud.instruction.transfer(
                Buffer.from(tree.root, 0, 32),
                Buffer.from(config.overrides?.message ?? message),
                index,
                {
                    accounts: {
                        authority: treeAdmin,
                        authorityPda: treeAuthorityPDA,
                        gummyrollProgram: Gummyroll.programId,
                        merkleRoll: treeAddress,
                        newOwner: newOwnerPubkey,
                        owner: treeAdminKeypair.publicKey,
                    },
                    signers,
                    remainingAccounts: proofPubkeys,
                }
            );
            const tx = new Transaction().add(transferIx);
            const transferTxId = await GummyrollCrud.provider.send(tx, signers, {
                commitment: "confirmed",
            });
            assert(transferTxId, "Failed to transfer an asset");
        }

        let treeKeypair: Keypair;
        beforeEach(async () => {
            const [computedTreeKeypair] = await createTree(
                treeAdminKeypair,
                MAX_DEPTH,
                MAX_BUFFER_SIZE
            );
            treeKeypair = computedTreeKeypair;
            await appendAsset(treeKeypair.publicKey, treeAdminKeypair, message);
        });
        it("changes the owner on the payload", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                treeAdminKeypair.publicKey,
                message,
                0
            );
            const newOwnerPubkey = Keypair.generate().publicKey;
            await transferAsset(
                treeKeypair.publicKey,
                treeAdminKeypair.publicKey,
                treeAdminKeypair,
                newOwnerPubkey,
                0
            );
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                newOwnerPubkey,
                message,
                0
            );
            const actualRoot = await getActualRoot(treeKeypair.publicKey);
            const expectedRoot = tree.root;
            expect(expectedRoot.compare(actualRoot)).to.equal(
                0,
                "On-chain root hash does not equal expected hash"
            );
        });
        it("fails if the message is modified", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                treeAdminKeypair.publicKey,
                message,
                0
            );
            const newOwnerPubkey = Keypair.generate().publicKey;
            try {
                await transferAsset(
                    treeKeypair.publicKey,
                    treeAdminKeypair.publicKey,
                    treeAdminKeypair,
                    newOwnerPubkey,
                    0,
                    {
                        overrides: { message: "mOdIfIeD mEsSaGe" },
                    }
                );
                assert(
                    false,
                    "Transaction should have failed since the message was modified"
                );
            } catch {
            }
            const actualRoot = await getActualRoot(treeKeypair.publicKey);
            const expectedRoot = tree.root;
            expect(expectedRoot.compare(actualRoot)).to.equal(
                0,
                "The transaction should have failed because the message was " +
                "modified, but never the less, the on-chain root hash changed."
            );
        });
        it("fails if someone other than the owner tries to transfer an asset", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                treeAdminKeypair.publicKey,
                message,
                0
            );
            const thiefKeypair = Keypair.generate();
            await Gummyroll.provider.connection.confirmTransaction(
                await Gummyroll.provider.connection.requestAirdrop(
                    thiefKeypair.publicKey,
                    2e9
                ),
                "confirmed"
            );
            try {
                await transferAsset(
                    treeKeypair.publicKey,
                    treeAdminKeypair.publicKey,
                    treeAdminKeypair,
                    thiefKeypair.publicKey,
                    0,
                    {
                        overrides: { signer: thiefKeypair },
                    }
                );
                assert(
                    false,
                    "Transaction should have failed since the signer was not the owner"
                );
            } catch {
            }
        });
    });
    describe("`Remove` instruction", () => {
        const message = "Message";

        async function removeAsset(
            treeAddress: PublicKey,
            treeAdminKeypair: Keypair,
            index: number,
            config: { overrides?: { leafHash?: Buffer; signer?: Keypair } } = {}
        ) {
            const proofPubkeys = getProofOfLeaf(tree, index).map(({ node }) => ({
                pubkey: new PublicKey(node),
                isSigner: false,
                isWritable: false,
            }));
            const [treeAuthorityPDA] = await getTreeAuthorityPDA(
                treeAddress,
                treeAdminKeypair.publicKey
            );
            const signers = [config.overrides?.signer ?? treeAdminKeypair];
            const root = Buffer.from(tree.root, 0, 32);
            const leafHash =
                config.overrides?.leafHash ??
                Buffer.from(tree.leaves[index].node, 0, 32);
            const transferIx = GummyrollCrud.instruction.remove(
                Array.from(root),
                Array.from(leafHash),
                index,
                {
                    accounts: {
                        authority: treeAdminKeypair.publicKey,
                        authorityPda: treeAuthorityPDA,
                        gummyrollProgram: Gummyroll.programId,
                        merkleRoll: treeAddress,
                    },
                    signers,
                    remainingAccounts: proofPubkeys,
                }
            );
            const tx = new Transaction().add(transferIx);
            const removeTxId = await GummyrollCrud.provider.send(tx, signers, {
                commitment: "confirmed",
            });
            assert(removeTxId, "Failed to remove an asset");
        }

        let treeKeypair: Keypair;
        beforeEach(async () => {
            const [computedTreeKeypair] = await createTree(
                treeAdminKeypair,
                MAX_DEPTH,
                MAX_BUFFER_SIZE
            );
            treeKeypair = computedTreeKeypair;
            await appendAsset(treeKeypair.publicKey, treeAdminKeypair, message);
        });
        it("removes the asset", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                treeAdminKeypair.publicKey,
                message,
                0
            );
            await removeAsset(treeKeypair.publicKey, treeAdminKeypair, 0);
            recomputeRootByRemovingLeafFromTreeAtIndex(0);
            const actualRoot = await getActualRoot(treeKeypair.publicKey);
            const expectedRoot = tree.root;
            expect(expectedRoot.compare(actualRoot)).to.equal(
                0,
                "On-chain root hash does not equal expected hash"
            );
        });
        it("fails if the leaf hash is incorrect", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                treeAdminKeypair.publicKey,
                message,
                0
            );
            try {
                await removeAsset(treeKeypair.publicKey, treeAdminKeypair, 0, {
                    overrides: { leafHash: Buffer.alloc(32, 0) },
                });
                assert(
                    false,
                    "Transaction should have failed since the leaf hash was wrong"
                );
            } catch {
            }
            const actualRoot = await getActualRoot(treeKeypair.publicKey);
            const expectedRoot = tree.root;
            expect(expectedRoot.compare(actualRoot)).to.equal(
                0,
                "The transaction should have failed because the leaf hash was " +
                "wrong, but never the less, the on-chain root hash changed."
            );
        });
        it("fails if someone other than the tree admin tries to remove a leaf", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                treeAdminKeypair.publicKey,
                message,
                0
            );
            const attackerKeypair = Keypair.generate();
            await Gummyroll.provider.connection.confirmTransaction(
                await Gummyroll.provider.connection.requestAirdrop(
                    attackerKeypair.publicKey,
                    2e9
                ),
                "confirmed"
            );
            try {
                await removeAsset(treeKeypair.publicKey, treeAdminKeypair, 0, {
                    overrides: { signer: attackerKeypair },
                });
                assert(
                    false,
                    "Transaction should have failed since the signer was not the owner"
                );
            } catch {
            }
        });
    });
});
