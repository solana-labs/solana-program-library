import { Keypair, PublicKey } from '@solana/web3.js';
import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountInstruction,
    createInitializeMintInstruction,
    createSyncNativeInstruction,
    createTransferCheckedInstruction,
    getAssociatedTokenAddress,
    TOKEN_PROGRAM_ID,
    TokenOwnerOffCurveError,
} from '../../src';

chai.use(chaiAsPromised);

describe('instructions', () => {
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
