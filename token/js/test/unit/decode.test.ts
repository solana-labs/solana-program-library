import { Keypair } from '@solana/web3.js';
import { expect } from 'chai';
import {
    createInitializeMintCloseAuthorityInstruction,
    createInitializePermanentDelegateInstruction,
    TOKEN_2022_PROGRAM_ID,
} from '../../src';

describe('spl-token-2022 instructions', () => {
    it('InitializeMintCloseAuthority', () => {
        const ix = createInitializeMintCloseAuthorityInstruction(
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(ix.programId).to.eql(TOKEN_2022_PROGRAM_ID);
        expect(ix.keys).to.have.length(1);
    });
    it('InitializePermanentDelegate', () => {
        const ix = createInitializePermanentDelegateInstruction(
            Keypair.generate().publicKey,
            Keypair.generate().publicKey,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(ix.programId).to.eql(TOKEN_2022_PROGRAM_ID);
        expect(ix.keys).to.have.length(1);
    });
});
