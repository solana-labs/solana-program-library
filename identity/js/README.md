# SPL Identity - JS client

SPL Identity is a Solana program, which adds the concept of Self-Sovereign Identity
to the Solana blockchain. It allows other Solana programs to add
"identity gate" functionality: A user can interact with the program
only if they are in possession of a valid identity,
certified by a trusted identity validator.

A user's personal identity information is not stored on-chain, rather, the identity
validator stores a hash of the data against a new account type called an "Identity Account"

This is the JS client for the Identity program.

## Usage

### Install
```
npm install @civic/spl-identity
```
or 
```
yarn add @civic/spl-identity
```

### Import
```
import { Identity } from '@civic/spl-identity';
```

### Creating a new Identity

Given a funded owner `Account`:

```
const identityAccount = await identity.createAccount(owner.publicKey);
```

### Get identity account information

```
const identityAccountInfo = await identity.getAccountInfo(identityAccount);

// A new identity has no attestations
expect(identityAccountInfo.attestation).to.be.undefined();
expect(identityAccountInfo.owner).to.deep.equal(owner.publicKey);
```

### Add an attestation
  
Given a funded IDV `Account`:
```
const attestation = new Uint8Array(
  // an attestation must be 32 bytes in length
  Buffer.from('my attestation'.padStart(32, ' '), 'utf-8')
);

await identity.attest(identityAccount, idv, attestation);
```
   
### Validate an attestation

```
const identityHasAttestation = await identity.hasAttestation(
    identityAccount,
    idv.publicKey,
    attestation
);

expect(identityHasAttestation).to.be.true;
```

## Commands

To build using TSDX, use:

```bash
yarn start
```

This builds to `/dist` using [Rollup](https://rollupjs.org), and runs the project in watch mode so any edits you save inside `src` causes a rebuild to `/dist`.

To do a one-off build, use `yarn build`.

To run tests, use `yarn test`.
