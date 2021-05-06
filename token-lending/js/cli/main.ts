/**
 * Exercises the token-lending program
 */

import { createLendingMarket, deployPorgram } from "./token-lending-test";

async function main() {
  // These test cases are designed to run sequentially and in the following order
  console.log("Run test: createLendingMarket");
  await deployPorgram();
  await createLendingMarket();
  console.log("Success\n");
}


main().catch((err) => console.log(err))
