/**
 * Exercises the token-swap program
 *
 * @flow
 */

import {
  loadPrograms,
  createNewTokenSwap,
  swap,
  deposit,
  withdraw,
} from './token-swap-test';

async function main() {
  // These test cases are designed to run sequentially and in the following order
  console.log('Run test: createNewToken');
  await loadPrograms();
  console.log('Run test: createNewToken');
  await createNewTokenSwap();
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
  })
  .then(() => process.exit());
