/**
 * Exercises the token-lending program
 */

import { loadPrograms } from "./token-lending-test";

async function main() {
  // These test cases are designed to run sequentially and in the following order
  console.log("Run test: loadPrograms");
  await loadPrograms();
  console.log("Success\n");
}

main().then(
  () => process.exit(),
  (err) => {
    console.error(err);
    process.exit(-1);
  }
);
