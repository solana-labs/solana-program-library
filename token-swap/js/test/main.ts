import {
  initializePoolRegistry,
  createAccountAndSwapAtomic,
  createTokenSwap,
  swap,
  depositAllTokenTypes,
  withdrawAllTokenTypes,
  depositSingleTokenTypeExactAmountIn,
  withdrawSingleTokenTypeExactAmountOut,
  createSecondTokenSwapForRouting,
  routedSwap,
  createAccountsAndRoutedSwapAtomic,
  createThirdTokenSwapForNative,
  swapNative,
  swapNativeToNewATALargeBoiTest,
} from './token-swap-test';

async function main() {
  //These test cases are designed to run sequentially and in the following order
  console.log('Run test: initialize pool registry');
  await initializePoolRegistry();
  console.log('Run test: createTokenSwap');
  await createTokenSwap();
  console.log('Run test: deposit all token types');
  await depositAllTokenTypes();
  console.log('Run test: withdraw all token types');
  await withdrawAllTokenTypes();
  console.log('Run test: swap');
  await swap();
  console.log('Run test: create account, approve, swap all at once');
  await createAccountAndSwapAtomic();
  console.log('Run test: deposit one exact amount in');
  await depositSingleTokenTypeExactAmountIn();
  console.log('Run test: withrdaw one exact amount out');
  await withdrawSingleTokenTypeExactAmountOut();

  console.log('Re-Run test: initialize pool registry');
  await initializePoolRegistry();
  console.log('Re-Run test: createTokenSwap');
  await createTokenSwap();
  console.log('Run test: create second token swap');
  await createSecondTokenSwapForRouting();
  console.log('Run test: routed swap');
  await routedSwap();
  console.log('Success\n');
  await createAccountsAndRoutedSwapAtomic();

  console.log('Run test: createThirdTokenSwap');
  await createThirdTokenSwapForNative();
  console.log('Run test: swapNative');
  await swapNative();
  console.log('Run test: swapNativeToNewATALargeBoiTest');
  await swapNativeToNewATALargeBoiTest();

  console.log('Success\n');
}

main()
  .catch(err => {
    console.error(err);
    process.exit(-1);
  })
  .then(() => process.exit());
