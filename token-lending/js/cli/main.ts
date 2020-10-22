/**
 * Exercises the token-lending program
 */

import { loadPrograms, createLendingReserve } from "./token-lending-test";

async function main() {
  // These test cases are designed to run sequentially and in the following order
  console.log("Run test: loadPrograms");
  await loadPrograms();
  console.log("Run test: createLendingReserve");
  await createLendingReserve();
  console.log("Success\n");
}

main().then(
  () => process.exit(),
  (err) => {
    console.error(err);
    process.exit(-1);
  }
);
