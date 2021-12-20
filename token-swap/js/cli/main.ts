import {
  createAccountAndSwapAtomic,
  createTokenSwap,
  swap,
  depositAllTokenTypes,
  withdrawAllTokenTypes,
  depositSingleTokenTypeExactAmountIn,
  withdrawSingleTokenTypeExactAmountOut,
} from './token-swap-test';
import {CurveType, Numberu64} from '../dist';

async function main() {
  // These test cases are designed to run sequentially and in the following order
  console.log('Run test: createTokenSwap (constant price)');
  await createTokenSwap(CurveType.ConstantPrice, new Numberu64(1));
  console.log(
    'Run test: createTokenSwap (constant product, used further in tests)',
  );
  await createTokenSwap(CurveType.ConstantProduct);
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
  console.log('Success\n');
}

main()
  .catch(err => {
    console.error(err);
    process.exit(-1);
  })
  .then(() => process.exit());
