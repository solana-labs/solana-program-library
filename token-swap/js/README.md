<h1 align="center">
  <br>
   <img width="300" src="https://github.com/step-finance/solana-program-library/blob/master/token-swap/js/logo.svg?raw=true" alt="step logo"/>
  <br>
</h1>

# @stepfinance/step-swap

## Installation

```
yarn add @stepfinance/step-swap
```

## Usage

**Querying Step Pool Registry**

```ts
import {
  TokenSwap as StepTokenSwap,
  STEP_SWAP_OWNER,
  STEP_SWAP_PROGRAM_ID,
} from '@stepfinance/step-swap';

const poolRegistry = await StepTokenSwap.loadPoolRegistry(
  connection,
  STEP_SWAP_OWNER,
  STEP_SWAP_PROGRAM_ID,
);

const poolAccountAddresses = poolRegistry.accounts.slice(
  0,
  poolRegistry.registrySize,
);
```
