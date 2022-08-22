import { createMemoInstruction, MEMO_PROGRAM_ID } from '../../src';
import { expect } from 'chai';
import { Keypair } from '@solana/web3.js';

test('instruction: no signers', () => {
    const ix = createMemoInstruction('this is a test memo', []);
    expect(ix.programId).to.eql(MEMO_PROGRAM_ID);
    expect(ix.keys).to.have.length(0);
    expect(ix.data).to.have.length(19);

    const ix2 = createMemoInstruction('this is a test');
    expect(ix2.programId).to.eql(MEMO_PROGRAM_ID);
    expect(ix2.keys).to.have.length(0);
    expect(ix2.data).to.have.length(14);
});

test('instruction: one signer', () => {
    const signer = new Keypair();
    const ix = createMemoInstruction('this is a test memo', [signer.publicKey]);
    expect(ix.programId).to.eql(MEMO_PROGRAM_ID);
    expect(ix.keys).to.have.length(1);
    expect(ix.data).to.have.length(19);
});

test('instruction: many signers', () => {
    const signer0 = new Keypair();
    const signer1 = new Keypair();
    const signer2 = new Keypair();
    const signer3 = new Keypair();
    const signer4 = new Keypair();
    const ix = createMemoInstruction('this is a test memo', [
        signer0.publicKey,
        signer1.publicKey,
        signer2.publicKey,
        signer3.publicKey,
        signer4.publicKey,
    ]);
    expect(ix.programId).to.eql(MEMO_PROGRAM_ID);
    expect(ix.keys).to.have.length(5);
    expect(ix.data).to.have.length(19);
});
