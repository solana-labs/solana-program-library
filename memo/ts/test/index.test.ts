import {expect} from 'chai';
import {Buffer} from 'buffer';

import {MemoProgram} from '../src/index';
import {TransactionInstruction, Keypair, PublicKey} from '@solana/web3.js';

describe('Memo', () => {
  it('Return programId', () => {
    const programId = MemoProgram.id();
    expect(programId.toString()).to.eq(
      'MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr',
    );
  });

  it('Check ProgramId Failure', () => {
    const newKey = new PublicKey('11111111111111111111111111111111');
    const idCheck = MemoProgram.checkId(newKey);

    expect(idCheck).to.eq(false);
  });

  it('Check ProgramId Success', () => {
    const newKey = new PublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr');
    const idCheck = MemoProgram.checkId(newKey);

    expect(idCheck).to.eq(true);
  });

  it('log memo without signers', () => {
    const sampleMemo = MemoProgram.buildMemo({memo: 'This is a sample memo'});

    const resultTransaction = new TransactionInstruction({
      data: Buffer.from('This is a sample memo'),
      keys: [],
      programId: MemoProgram.programId,
    });

    expect(sampleMemo.data.toString()).to.eq(resultTransaction.data.toString());
  });

  it('log memo with signers', () => {
    const account1 = new Keypair();
    const account2 = new Keypair();
    const account3 = new Keypair();
    const sampleMemo = MemoProgram.buildMemo({
      memo: 'This is a sample memo',
      signer_public_keys: [
        account1.publicKey,
        account2.publicKey,
        account3.publicKey,
      ],
    });
    const resultTransaction = new TransactionInstruction({
      data: Buffer.from('This is a sample memo'),
      keys: [
        {pubkey: account1.publicKey, isSigner: true, isWritable: true},
        {pubkey: account2.publicKey, isSigner: true, isWritable: true},
        {pubkey: account3.publicKey, isSigner: true, isWritable: true},
      ],
      programId: MemoProgram.programId,
    });

    expect(sampleMemo.data.toString()).to.eq(resultTransaction.data.toString());
  });
});
