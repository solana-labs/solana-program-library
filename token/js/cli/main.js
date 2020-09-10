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
  transfer2,
  approveRevoke,
  failOnApproveOverspend,
  setAuthority,
  mintTo,
  mintTo2,
  multisig,
  burn,
  burn2,
  freezeThawAccount,
  closeAccount,
  nativeToken,
} from './token-test';

async function main() {
  console.log('Run test: loadTokenProgram');
  await loadTokenProgram();
  console.log('Run test: createMint');
  await createMint();
  console.log('Run test: createAccount');
  await createAccount();
  console.log('Run test: mintTo');
  await mintTo();
  console.log('Run test: mintTo2');
  await mintTo2();
  console.log('Run test: transfer');
  await transfer();
  console.log('Run test: transfer2');
  await transfer2();
  console.log('Run test: approveRevoke');
  await approveRevoke();
  console.log('Run test: failOnApproveOverspend');
  await failOnApproveOverspend();
  console.log('Run test: setAuthority');
  await setAuthority();
  console.log('Run test: burn');
  await burn();
  console.log('Run test: burn2');
  await burn2();
  console.log('Run test: freezeThawAccount');
  await freezeThawAccount();
  console.log('Run test: closeAccount');
  await closeAccount();
  console.log('Run test: multisig');
  await multisig();
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
