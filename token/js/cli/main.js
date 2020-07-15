/**
 * Exercises the token program
 *
 * @flow
 */

import {
  loadTokenProgram,
  createMint,
  createAccount,
  transfer,
  approveRevoke,
  invalidApprove,
  failOnApproveOverspend,
  setOwner,
  mintTo,
  multisig,
  burn,
  failOnCloseAccount,
  nativeToken,
} from './token-test';

async function main() {
  console.log('Run test: loadTokenProgram');
  await loadTokenProgram();
  console.log('Run test: createMint');
  await createMint();
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
  console.log('Run test: multisig');
  await multisig();
  console.log('Run test: burn');
  await burn();
  console.log('Run test: failOnCloseAccount');
  await failOnCloseAccount();
  console.log('Run test: nativeToken');
  await nativeToken();
  console.log('Success\n');
}

main()
  .catch(err => {
    console.error(err);
    process.exit(-1);
  })
  .then(() => process.exit());
