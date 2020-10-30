import { createOwnerAccount, loadIdentityProgram } from './utils';
import { Identity } from '../src/client/identity';
import { Account } from '@solana/web3.js';

describe('Identity', function() {
  this.timeout(60000);

  let identity: Identity;
  let owner: Account;

  before('create owner account', async () => {
    owner = await createOwnerAccount();
  });

  before('load program', async () => {
    identity = await loadIdentityProgram(owner);
  });

  it('should register an identity account', async () => {
    const identityAccount = await identity.createAccount(owner.publicKey);

    console.log(identityAccount);
  });
});
