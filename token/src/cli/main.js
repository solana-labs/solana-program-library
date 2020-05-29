/**
 * Exercises the token program
 *
 * @flow
 */

import {
  loadTokenProgram,
  createNewToken,
  createNewTokenAccount,
  transfer,
  approveRevoke,
  invalidApprove,
  failOnApproveOverspend,
  setOwner,
} from './token-test';

async function main() {
  console.log('Run test: loadTokenProgram');
  await loadTokenProgram();
  console.log('Run test: createNewToken');
  await createNewToken();
  console.log('Run test: createNewTokenAccount');
  await createNewTokenAccount();
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
  console.log('Success\n');
}

main()
  .catch(err => {
    console.error(err);
  })
  .then(() => process.exit());
