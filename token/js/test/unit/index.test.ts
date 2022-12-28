import { Keypair, PublicKey } from '@solana/web3.js';
import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountInstruction,
    createReallocateInstruction,
    createInitializeMintInstruction,
    createInitializeMint2Instruction,
    createSyncNativeInstruction,
    createTransferCheckedInstruction,
    getAssociatedTokenAddress,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    TokenInstruction,
    TokenOwnerOffCurveError,
    getAccountLen,
    ExtensionType,
    getAssociatedTokenAddressSync,
    createInitializeAccount2Instruction,
    createInitializeAccount3Instruction,
    createAmountToUiAmountInstruction,
    createUiAmountToAmountInstruction,
} from '../../src';

chai.use(chaiAsPromised);

describe('spl-token instructions', () => {
    it('TransferChecked', () => {
        const ix = createTransferCheckedInstruction(
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
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

    it('InitializeMint2', () => {
        const ix = createInitializeMint2Instruction(
            Keypair.generate().publicKey,
            9,
            Keypair.generate().publicKey,
            null
        );
        expect(ix.programId).to.eql(TOKEN_PROGRAM_ID);
        expect(ix.keys).to.have.length(1);
    });

    it('SyncNative', () => {
        const ix = createSyncNativeInstruction(Keypair.generate().publicKey);
        expect(ix.programId).to.eql(TOKEN_PROGRAM_ID);
        expect(ix.keys).to.have.length(1);
    });

    it('InitializeAccount2', () => {
        const ix = createInitializeAccount2Instruction(
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey
        );
        expect(ix.programId).to.eql(TOKEN_PROGRAM_ID);
        expect(ix.keys).to.have.length(3);
    });

    it('InitializeAccount3', () => {
        const ix = createInitializeAccount3Instruction(
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey
        );
        expect(ix.programId).to.eql(TOKEN_PROGRAM_ID);
        expect(ix.keys).to.have.length(2);
    });
});

describe('spl-token-2022 instructions', () => {
    it('TransferChecked', () => {
        const ix = createTransferCheckedInstruction(
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            1,
            9,
            [],
            TOKEN_2022_PROGRAM_ID
        );
        expect(ix.programId).to.eql(TOKEN_2022_PROGRAM_ID);
        expect(ix.keys).to.have.length(4);
    });

    it('InitializeMint', () => {
        const ix = createInitializeMintInstruction(
            Keypair.generate().publicKey,
            9,
            Keypair.generate().publicKey,
            null,
            TOKEN_2022_PROGRAM_ID
        );
        expect(ix.programId).to.eql(TOKEN_2022_PROGRAM_ID);
        expect(ix.keys).to.have.length(2);
    });

    it('InitializeMint2', () => {
        const ix = createInitializeMint2Instruction(
            Keypair.generate().publicKey,
            9,
            Keypair.generate().publicKey,
            null,
            TOKEN_2022_PROGRAM_ID
        );
        expect(ix.programId).to.eql(TOKEN_2022_PROGRAM_ID);
        expect(ix.keys).to.have.length(1);
    });

    it('SyncNative', () => {
        const ix = createSyncNativeInstruction(Keypair.generate().publicKey, TOKEN_2022_PROGRAM_ID);
        expect(ix.programId).to.eql(TOKEN_2022_PROGRAM_ID);
        expect(ix.keys).to.have.length(1);
    });

    it('Reallocate', () => {
        const publicKey = Keypair.generate().publicKey;
        const extensionTypes = [ExtensionType.MintCloseAuthority, ExtensionType.TransferFeeConfig];
        const ix = createReallocateInstruction(publicKey, publicKey, extensionTypes, publicKey);
        expect(ix.programId).to.eql(TOKEN_2022_PROGRAM_ID);
        expect(ix.keys).to.have.length(4);
        console.error(ix.data);
        expect(ix.data[0]).to.eql(TokenInstruction.Reallocate);
        expect(ix.data[1]).to.eql(extensionTypes[0]);
        expect(ix.data[3]).to.eql(extensionTypes[1]);
    });

    it('AmountToUiAmount', () => {
        const ix = createAmountToUiAmountInstruction(Keypair.generate().publicKey, 22, TOKEN_2022_PROGRAM_ID);
        expect(ix.programId).to.eql(TOKEN_2022_PROGRAM_ID);
        expect(ix.keys).to.have.length(1);
    });

    it('UiAmountToAmount', () => {
        const ix = createUiAmountToAmountInstruction(Keypair.generate().publicKey, '22', TOKEN_2022_PROGRAM_ID);
        expect(ix.programId).to.eql(TOKEN_2022_PROGRAM_ID);
        expect(ix.keys).to.have.length(1);
    });
});

describe('spl-associated-token-account instructions', () => {
    it('create', () => {
        const ix = createAssociatedTokenAccountInstruction(
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            Keypair.generate().publicKey
        );
        expect(ix.programId).to.eql(ASSOCIATED_TOKEN_PROGRAM_ID);
        expect(ix.keys).to.have.length(6);
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

    it('getAssociatedTokenAddressSync matches getAssociatedTokenAddress', async () => {
        const asyncAssociatedPublicKey = await getAssociatedTokenAddress(
            new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z'),
            new PublicKey('B8UwBUUnKwCyKuGMbFKWaG7exYdDk2ozZrPg72NyVbfj')
        );
        const associatedPublicKey = getAssociatedTokenAddressSync(
            new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z'),
            new PublicKey('B8UwBUUnKwCyKuGMbFKWaG7exYdDk2ozZrPg72NyVbfj')
        );
        expect(associatedPublicKey.toString()).to.eql(
            new PublicKey('DShWnroshVbeUp28oopA3Pu7oFPDBtC1DBmPECXXAQ9n').toString()
        );
        expect(asyncAssociatedPublicKey.toString()).to.eql(associatedPublicKey.toString());

        expect(function () {
            getAssociatedTokenAddressSync(
                new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z'),
                associatedPublicKey
            );
        }).to.throw(TokenOwnerOffCurveError);

        const asyncAssociatedPublicKey2 = await getAssociatedTokenAddress(
            new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z'),
            asyncAssociatedPublicKey,
            true
        );
        const associatedPublicKey2 = getAssociatedTokenAddressSync(
            new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z'),
            associatedPublicKey,
            true
        );
        expect(associatedPublicKey2.toString()).to.eql(
            new PublicKey('F3DmXZFqkfEWFA7MN2vDPs813GeEWPaT6nLk4PSGuWJd').toString()
        );
        expect(asyncAssociatedPublicKey2.toString()).to.eql(associatedPublicKey2.toString());
    });
});

describe('extensionType', () => {
    it('calculates size', () => {
        expect(getAccountLen([ExtensionType.MintCloseAuthority, ExtensionType.TransferFeeConfig])).to.eql(314);
        expect(getAccountLen([])).to.eql(165);
        expect(getAccountLen([ExtensionType.ImmutableOwner])).to.eql(170);
        expect(getAccountLen([ExtensionType.PermanentDelegate])).to.eql(202);
    });
});
