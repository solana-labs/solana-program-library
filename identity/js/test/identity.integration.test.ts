import chai from 'chai';
import dirtyChai from 'dirty-chai';
import { createAccount, loadIdentityProgram } from './utils';
import { ATTESTATION_SIZE, Identity } from '../src/client/identity';
import { Account, PublicKey } from '@solana/web3.js';

chai.use(dirtyChai);
const { expect } = chai;

const attestation = 'hello'.padStart(ATTESTATION_SIZE, ' ');

let identity: Identity;
let owner: Account;
let idv: Account;

let identityAccount: PublicKey;

describe('Identity', function() {
  this.timeout(60000);

  before('create owner and idv accounts', async () => {
    owner = await createAccount();
    idv = await createAccount();
  });

  before('load program', async () => {
    identity = await loadIdentityProgram(owner);
  });

  it('should register an identity account', async () => {
    identityAccount = await identity.createAccount(owner.publicKey);

    console.log(identityAccount.toBase58());
  });

  it('should get the identity account info', async () => {
    const identityAccountInfo = await identity.getAccountInfo(identityAccount);

    expect(identityAccountInfo.attestation).to.be.undefined();
    expect(identityAccountInfo.owner).to.deep.equal(owner.publicKey);
  });

  it('should add an attestation', async () => {
    await identity.attest(identityAccount, idv, attestation);

    const identityHasAttestation = await identity.hasAttestation(
      identityAccount,
      idv.publicKey,
      attestation
    );

    expect(identityHasAttestation).to.be.true();
  });
});
