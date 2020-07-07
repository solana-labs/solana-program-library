/**
 * Exercises the token-swap program
 *
 * @flow
 */

import {
  loadPrograms,
  createTokenSwap,
  swap,
  deposit,
  withdraw,
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
  console.log('Success\n');
}

main()
  .catch(err => {
    console.error(err);
    process.exit(-1);
  })
  .then(() => process.exit());
