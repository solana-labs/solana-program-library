// @flow
import {expect} from 'chai';
import {Account} from '@solana/web3.js';

import {Token, TOKEN_PROGRAM_ID} from '../client/token';

describe('Token', () => {
  it('createTransfer', () => {
    const ix = Token.createTransferCheckedInstruction(
      TOKEN_PROGRAM_ID,
      new Account().publicKey,
      new Account().publicKey,
      new Account().publicKey,
      new Account().publicKey,
      [],
      1,
      9,
    );
    expect(ix.programId).to.eql(TOKEN_PROGRAM_ID);
    expect(ix.keys).to.have.length(4);
  });

  it('createInitMint', () => {
    const ix = Token.createInitMintInstruction(
      TOKEN_PROGRAM_ID,
      new Account().publicKey,
      9,
      new Account().publicKey,
      null,
    );
    expect(ix.programId).to.eql(TOKEN_PROGRAM_ID);
    expect(ix.keys).to.have.length(2);
  });
});
