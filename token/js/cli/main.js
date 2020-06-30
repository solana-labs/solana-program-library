/**
 * Exercises the token program
 *
 * @flow
 */

import {
  loadTokenProgram,
  createToken,
  createAccount,
  transfer,
  approveRevoke,
  invalidApprove,
  failOnApproveOverspend,
  setOwner,
  mintTo,
  burn,
} from './token-test';

async function main() {
  console.log('Run test: loadTokenProgram');
  await loadTokenProgram('../target/bpfel-unknown-unknown/release/spl_token.so');
  console.log('Run test: createToken');
  await createToken();
  console.log('Run test: createAccount');
  await createAccount();
  console.log('Run test: transfer');
  await transfer();
  console.log('Run test: approveRevoke');
  await approveRevoke();
  console.log('Run test: invalidApprove');
  await invalidApprove();
  console.log('Run test: failOnApproveOverspend');
  await failOnApproveOverspend();
  console.log('Run test: setOwner');
  await setOwner();
  console.log('Run test: mintTo');
  await mintTo();
  console.log('Run test: burn');
  await burn();
  console.log('Success\n');
}

main()
  .catch(err => {
    console.error(err);
  })
  .then(() => process.exit());
