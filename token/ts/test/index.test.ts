import {Connection, Keypair, LAMPORTS_PER_SOL, PublicKey} from '@solana/web3.js';
import chai, {expect} from 'chai';
import chaiAsPromised from 'chai-as-promised';
import {
    approve,
    approveChecked,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    burn,
    burnChecked,
    closeAccount,
    createAssociatedTokenAccountInstruction,
    createInitializeMintInstruction,
    createMint,
    createMultisig,
    createSyncNativeInstruction,
    createTransferCheckedInstruction,
    freezeAccount,
    getAccountInfo,
    getAssociatedTokenAddress,
    getMultisigInfo,
    getOrCreateAssociatedTokenAccount,
    mintTo,
    revoke,
    thawAccount,
    TOKEN_PROGRAM_ID,
    TokenOwnerOffCurveError,
    transfer,
    transferChecked,
} from '../src';

chai.use(chaiAsPromised);

describe('instructions', () => {
    it('TransferChecked', () => {
        const ix = createTransferCheckedInstruction(
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            [],
            1,
            9
        );
        expect(ix.programId).to.eql(TOKEN_PROGRAM_ID);
        expect(ix.keys).to.have.length(4);
    });

    it('InitializeMint', () => {
        const ix = createInitializeMintInstruction(Keypair.generate().publicKey, 9, Keypair.generate().publicKey, null);
        expect(ix.programId).to.eql(TOKEN_PROGRAM_ID);
        expect(ix.keys).to.have.length(2);
    });

    it('SyncNative', () => {
        const ix = createSyncNativeInstruction(Keypair.generate().publicKey);
        expect(ix.programId).to.eql(TOKEN_PROGRAM_ID);
        expect(ix.keys).to.have.length(1);
    });

    it('AssociatedTokenAccount', () => {
        const ix = createAssociatedTokenAccountInstruction(
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey
        );
        expect(ix.programId).to.eql(ASSOCIATED_TOKEN_PROGRAM_ID);
        expect(ix.keys).to.have.length(7);
    });
});

describe('state', () => {
    it('getAssociatedTokenAddress', async () => {
        const associatedPublicKey = await getAssociatedTokenAddress(
            new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z'),
            new PublicKey('B8UwBUUnKwCyKuGMbFKWaG7exYdDk2ozZrPg72NyVbfj')
        );
        expect(associatedPublicKey.toString()).to.eql(
            new PublicKey('DShWnroshVbeUp28oopA3Pu7oFPDBtC1DBmPECXXAQ9n').toString()
        );
        await expect(
            getAssociatedTokenAddress(
                new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z'),
                associatedPublicKey
            )
        ).to.be.rejectedWith(TokenOwnerOffCurveError);

        const associatedPublicKey2 = await getAssociatedTokenAddress(
            new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z'),
            associatedPublicKey,
            true
        );
        expect(associatedPublicKey2.toString()).to.eql(
            new PublicKey('F3DmXZFqkfEWFA7MN2vDPs813GeEWPaT6nLk4PSGuWJd').toString()
        );
    });
});

describe('live', () => {
    if (process.env.TEST_LIVE) {
        const connection = new Connection(
            'http://127.0.0.1:8899',
            'confirmed'
        );

        const payer = Keypair.generate();
        const fromWallet = Keypair.generate();
        const toWallet  = Keypair.generate();
        const freezeAuthority = Keypair.generate();
        const mintAuthority = Keypair.generate();

        before(async () => {
            const airdropSignature = await connection.requestAirdrop(
                payer.publicKey,
                LAMPORTS_PER_SOL,
            );
            return connection.confirmTransaction(airdropSignature);
        })

        it('transfer', async () => {
            const mint = await createMint(
                connection,
                payer,
                mintAuthority.publicKey,
                freezeAuthority.publicKey,
                9
            )

            const fromTokenAccount = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                fromWallet.publicKey
            );

            const toTokenAccount = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                toWallet.publicKey
            );

            await mintTo(
                connection,
                payer,
                mint,
                fromTokenAccount.address,
                mintAuthority,
                [],
                1000000000
            );

            const toTokenAccountInfoPreTransfer = await getAccountInfo(
                connection,
                toTokenAccount.address
            )

            expect(toTokenAccountInfoPreTransfer.amount).to.eql(BigInt(0));

            await transfer(
                connection,
                payer,
                fromTokenAccount.address,
                toTokenAccount.address,
                fromWallet,
                [],
                1000000000
            );

            const toTokenAccountInfoPostTransfer = await getAccountInfo(
                connection,
                toTokenAccount.address
            )

            expect(toTokenAccountInfoPostTransfer.amount).to.eql(BigInt(1000000000));

            await expect(transfer(
                connection,
                payer,
                fromTokenAccount.address,
                toTokenAccount.address,
                fromWallet,
                [],
                1000000000
            )).to.be.rejected;

            await transferChecked(
                connection,
                payer,
                toTokenAccount.address,
                mint,
                fromTokenAccount.address,
                toWallet,
                [],
                1,
                9
            );

            const fromTokenAccountInfoPostTransferChecked = await getAccountInfo(
                connection,
                fromTokenAccount.address
            )

            expect(fromTokenAccountInfoPostTransferChecked.amount).to.eql(BigInt(1));

            await expect(transferChecked(
                connection,
                payer,
                toTokenAccount.address,
                mint,
                fromTokenAccount.address,
                toWallet,
                [],
                1000000000,
                9
            )).to.be.rejected;
        });

        it('burn', async () => {
            const mint = await createMint(
                connection,
                payer,
                mintAuthority.publicKey,
                freezeAuthority.publicKey,
                9
            )

            const fromTokenAccount = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                fromWallet.publicKey
            );

            await mintTo(
                connection,
                payer,
                mint,
                fromTokenAccount.address,
                mintAuthority,
                [],
                1000000000
            );

            const fromTokenAccountInfoPreBurn = await getAccountInfo(
                connection,
                fromTokenAccount.address
            )

            expect(fromTokenAccountInfoPreBurn.amount).to.eql(BigInt(1000000000));

            await burn(
                connection,
                payer,
                fromTokenAccount.address,
                mint,
                fromWallet,
                [],
                1
            )

            const fromTokenAccountInfoPostBurn = await getAccountInfo(
                connection,
                fromTokenAccount.address
            )

            expect(fromTokenAccountInfoPostBurn.amount).to.eql(BigInt(999999999));

            await expect(burn(
                connection,
                payer,
                fromTokenAccount.address,
                mint,
                fromWallet,
                [],
                1000000000
            )).to.be.rejected;

            await burnChecked(
                connection,
                payer,
                fromTokenAccount.address,
                mint,
                fromWallet,
                [],
                1,
                9
            )

            const fromTokenAccountInfoPostBurnChecked = await getAccountInfo(
                connection,
                fromTokenAccount.address
            )

            expect(fromTokenAccountInfoPostBurnChecked.amount).to.eql(BigInt(999999998));

            await expect(burnChecked(
                connection,
                payer,
                fromTokenAccount.address,
                mint,
                fromWallet,
                [],
                1000000000,
                9
            )).to.be.rejected;
        })

        it('freeze', async () => {
            const mint = await createMint(
                connection,
                payer,
                mintAuthority.publicKey,
                freezeAuthority.publicKey,
                9
            )

            const fromTokenAccount = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                fromWallet.publicKey
            );

            const toTokenAccount = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                toWallet.publicKey
            );

            await mintTo(
                connection,
                payer,
                mint,
                fromTokenAccount.address,
                mintAuthority,
                [],
                1000000000
            );

            const fromTokenAccountInfo = await getAccountInfo(
                connection,
                fromTokenAccount.address
            )

            expect(fromTokenAccountInfo.amount).to.eql(BigInt(1000000000));

            await freezeAccount(
                connection,
                payer,
                fromTokenAccount.address,
                mint,
                freezeAuthority,
                []
            )

            await expect(transfer(
                connection,
                payer,
                fromTokenAccount.address,
                toTokenAccount.address,
                fromWallet,
                [],
                1000000000)
            ).to.be.rejected;

            await thawAccount(
                connection,
                payer,
                fromTokenAccount.address,
                mint,
                freezeAuthority,
                []
            )

            await transfer(
                connection,
                payer,
                fromTokenAccount.address,
                toTokenAccount.address,
                fromWallet,
                [],
                1000000000
            )

            const toTokenAccountInfoPostTransfer = await getAccountInfo(
                connection,
                toTokenAccount.address
            )

            expect(toTokenAccountInfoPostTransfer.amount).to.eql(BigInt(1000000000));
        })

        it('approvalLifecycle', async() => {
            const mint = await createMint(
                connection,
                payer,
                mintAuthority.publicKey,
                freezeAuthority.publicKey,
                9
            )

            const fromTokenAccount = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                fromWallet.publicKey
            );

            const toTokenAccount = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                toWallet.publicKey
            );

            await mintTo(
                connection,
                payer,
                mint,
                fromTokenAccount.address,
                mintAuthority,
                [],
                1000000000
            );

            const fromTokenAccountInfo = await getAccountInfo(
                connection,
                fromTokenAccount.address
            )

            expect(fromTokenAccountInfo.amount).to.eql(BigInt(1000000000));

            await approve(
                connection,
                payer,
                fromTokenAccount.address,
                toWallet.publicKey,
                fromWallet,
                [],
                100
            )

            const toTokenAccountInfoPreTransfer = await getAccountInfo(
                connection,
                toTokenAccount.address
            )

            expect(toTokenAccountInfoPreTransfer.amount).to.eql(BigInt(0));

            // ToWallet can transfer from the fromAccount because it is approved
            await transfer(
                connection,
                payer,
                fromTokenAccount.address,
                toTokenAccount.address,
                toWallet,
                [],
                100
            )

            const toTokenAccountInfoPostTransfer = await getAccountInfo(
                connection,
                toTokenAccount.address
            )

            expect(toTokenAccountInfoPostTransfer.amount).to.eql(BigInt(100));

            // Attempting to move more than you are approved for will fail
            await expect(transfer(
                connection,
                payer,
                fromTokenAccount.address,
                toTokenAccount.address,
                toWallet,
                [],
                100
            )).to.be.rejected;

            await approveChecked(
                connection,
                payer,
                mint,
                fromTokenAccount.address,
                toWallet.publicKey,
                fromWallet,
                [],
                1000,
                9
            )

            await transfer(
                connection,
                payer,
                fromTokenAccount.address,
                toTokenAccount.address,
                toWallet,
                [],
                100
            )

            const toTokenAccountInfo = await getAccountInfo(
                connection,
                toTokenAccount.address
            )

            expect(toTokenAccountInfo.amount).to.eql(BigInt(200));

            await revoke(
                connection,
                payer,
                fromTokenAccount.address,
                fromWallet,
                []
            )

            // Won't be able to transfer after the account has revoked access
            await expect(transfer(
                connection,
                payer,
                fromTokenAccount.address,
                toTokenAccount.address,
                toWallet,
                [],
                100
            )).to.be.rejected;
        })

        it('accountLifecycle', async () => {
            const mint = await createMint(
                connection,
                payer,
                mintAuthority.publicKey,
                freezeAuthority.publicKey,
                9
            )

            const fromTokenAccount = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                fromWallet.publicKey
            );

            const fromAccount = await getAccountInfo(
                connection,
                fromTokenAccount.address
            );
            expect(fromAccount.mint.toBase58()).to.eql(mint.toBase58());
            expect(fromAccount.owner.toBase58()).to.eql(fromWallet.publicKey.toBase58());

            const associatedTokenAddress = await getAssociatedTokenAddress(
                mint,
                fromWallet.publicKey
            )

            expect(associatedTokenAddress.toBase58()).to.eql(fromTokenAccount.address.toBase58());

            await closeAccount(
                connection,
                payer,
                fromTokenAccount.address,
                payer.publicKey,
                fromWallet,
                []
            )

            await expect(
                getAccountInfo(
                    connection,
                    fromTokenAccount.address
                )
            ).to.be.rejected;
        })

        it('multisig', async () => {
            const signer1 = Keypair.generate();
            const signer2 = Keypair.generate();
            const signer3 = Keypair.generate();
            const signer4 = Keypair.generate();
            const signer5 = Keypair.generate();

            const multisigKey = await createMultisig(
                connection,
                payer,
                [
                    signer1.publicKey,
                    signer2.publicKey,
                    signer3.publicKey,
                    signer4.publicKey,
                    signer5.publicKey
                ],
                3
            )

            const mint = await createMint(
                connection,
                payer,
                multisigKey,
                multisigKey,
                9
            )

            const fromTokenAccount = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                fromWallet.publicKey
            );

            // can only mint when there are at least 3 signers
            await expect(mintTo(
                connection,
                payer,
                mint,
                fromTokenAccount.address,
                multisigKey,
                [],
                1000000000
            )).to.be.rejected;

            await expect(mintTo(
                connection,
                payer,
                mint,
                fromTokenAccount.address,
                multisigKey,
                [
                    signer1,
                    signer2
                ],
                1000000000
            )).to.be.rejected;

            await mintTo(
                connection,
                payer,
                mint,
                fromTokenAccount.address,
                multisigKey,
                [
                    signer1,
                    signer2,
                    signer3
                ],
                1000000000
            );

            const fromTokenAccountInfo = await getAccountInfo(
                connection,
                fromTokenAccount.address
            )

            expect(fromTokenAccountInfo.amount).to.eql(BigInt(1000000000));

            const multisigInfo = await getMultisigInfo(
                connection,
                multisigKey
            )

            expect(multisigInfo.address.toBase58()).to.eql(multisigKey.toBase58());
            expect(multisigInfo.signer1.toBase58()).to.eql(signer1.publicKey.toBase58());
            expect(multisigInfo.signer2.toBase58()).to.eql(signer2.publicKey.toBase58());
            expect(multisigInfo.signer3.toBase58()).to.eql(signer3.publicKey.toBase58());
            expect(multisigInfo.signer4.toBase58()).to.eql(signer4.publicKey.toBase58());
            expect(multisigInfo.signer5.toBase58()).to.eql(signer5.publicKey.toBase58());
        })
    }
});
