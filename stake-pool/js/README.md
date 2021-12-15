# TypeScript bindings for stake-pool program

For use with both node.js and in-browser.

## Installation

```
npm install
```

## Build and run

In the `js` folder:

```
npm run compile
npm run lint
node dist/index.js
```

## Test

```
npm run compile
npm test
```

Sample output:

```
> stake-pool-js@0.0.1 test
> ./node_modules/mocha/bin/mocha -p ./dist


  schema.decode
    StakePoolAccount
      ✓ should successfully decode StakePoolAccount account data
    ValidatorListAccount
      ✓ should successfully decode ValidatorListAccount account data
      ✓ should successfully decode ValidatorListAccount with nonempty ValidatorInfo

  index.ts/PrettyPrintPubkey
    ✓ should successfully pretty print a pubkey


  4 passing (610ms)
  ```