/**
 * Hello world
 */

import {
  establishConnection,
  establishPayer,
  loadProgram,
  sayHello,
  reportHellos,
  reportOwner,
  initializeOwnership,
  transferOwnershipToGreeted,
  transferOwnershipToPayer,
  reportPaused,
  pauseByPayer,
  pauseByGreeted,
  resumeByPayer,
  resumeByGreeted,
} from './hello_world';
import * as App from './hello_world';

async function main() {
  console.log("Let's say hello to a Solana account...");

  // Establish connection to the cluster
  await establishConnection();

  // Determine who pays for the fees, this is also the initial owner
  await establishPayer();

  // Load the program if not already loaded
  await loadProgram();

  // Say hello to an account
  await sayHello();

  // Find out how many times that account has been greeted
  await reportHellos();

  await reportOwner();
  await initializeOwnership();
  await reportOwner();
  await reportPaused();

  // Pause the program and say hello again
  const clientError = Error('CLIENT ERROR');
  try {
    await pauseByGreeted();
    console.log('CLIENT ERROR: Unexpected result, the Greeted should not be able to pause the program.')
    throw clientError;
  } catch (err) {
    if (err == clientError) {
        throw err;
    }
    console.log('As expected, the Greeted cannot pause the program.')
  }
  await reportPaused();
  await pauseByPayer();
  await reportPaused();

  await sayHello();
  await reportHellos();

  // Transfer ownership from Payer to Greeted
  await reportOwner();
  await transferOwnershipToGreeted();
  await reportOwner();

  try {
    await resumeByPayer();
    console.log('CLIENT ERROR: Unepxected result, the Payer should no longer by able to resume the program.')
    throw clientError;
  } catch (err) {
    if (err == clientError) {
        throw err;
    }
    console.log('As expected, the Payer can no longer resume the program.')
  }
  await reportPaused();
  await resumeByGreeted();
  await reportPaused();

  await sayHello();
  await reportHellos();

  // Restore ownership to Payer from Greeted
  await reportOwner();
  await transferOwnershipToPayer();
  await reportOwner();

  console.log('Success');
}

main().then(
  () => process.exit(),
  err => {
    console.error(err);
    process.exit(-1);
  },
);
