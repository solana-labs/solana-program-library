/**
 * Exercises the token-swap program
 *
 * @flow
 */

import {
  loadPrograms,
  createAccountAndSwapAtomic,
  createTokenSwap,
  swap,
  deposit,
  withdraw,
  depositOneExactIn,
  withdrawOneExactOut,
} from './token-swap-test';

async function main() {
  // These test cases are designed to run sequentially and in the following order
  console.log('Run test: loadPrograms');
  await loadPrograms();
  console.log('Run test: createTokenSwap');
  await createTokenSwap();
  console.log('Run test: deposit');
  await deposit();
  console.log('Run test: withdraw');
  await withdraw();
  console.log('Run test: swap');
  await swap();
  console.log('Run test: create account, approve, swap all at once');
  await createAccountAndSwapAtomic();
  console.log('Success\n');
  console.log('Run test: deposit one exact amount in');
  await depositOneExactIn();
  console.log('Run test: withrdaw one exact amount out');
  await withdrawOneExactOut();
}

main()
  .catch(err => {
    console.error(err);
    process.exit(-1);
  })
  .then(() => process.exit());
