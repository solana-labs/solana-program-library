import { Blockchain } from './blockchain';
import { assert } from './util';

async function main() {
    // --------------------------------------- init

    const bc = new Blockchain();
    await bc.getConnection();
    await bc.initLendingMarket();
    await bc.initReserve(bc.tokenA, 10000, 4000);
    await bc.initReserve(bc.tokenB, 500, 100); //get 100 lpSOL
    await bc.initObligation();
    await bc.calcAndPrintMetrics();

    // check user lost tokens
    assert(bc.metrics.tokenAUserBalance.value.uiAmount == 10000 - 4000);
    assert(bc.metrics.tokenBUserBalance.value.uiAmount == 500 - 100);
    // check protocol gained tokens
    assert(bc.metrics.tokenAProtocolBalance.value.uiAmount == 4000);
    assert(bc.metrics.tokenBProtocolBalance.value.uiAmount == 100);
    // check user was issued LP tokens in return
    assert(bc.metrics.tokenALPUserBalance.value.uiAmount == 4000);
    assert(bc.metrics.tokenBLPUserBalance.value.uiAmount == 100);
    // check total liquidity available
    assert(bc.metrics.reserveAState.data.liquidity.availableAmount == 4000n);
    assert(bc.metrics.reserveBState.data.liquidity.availableAmount == 100n);

    // --------------------------------------- depositing / withdrawing liquidity

    await bc.depositReserveLiquidity(bc.tokenA, 2000);
    await bc.redeemReserveCollateral(bc.tokenA, 1000);
    await bc.depositReserveLiquidity(bc.tokenB, 100);
    await bc.redeemReserveCollateral(bc.tokenB, 50);
    await bc.calcAndPrintMetrics();

    // check changes in balances add up
    assert(bc.metrics.tokenAUserBalance.value.uiAmount == 10000 - 4000 - 2000 + 1000);
    assert(bc.metrics.tokenAProtocolBalance.value.uiAmount == 4000 + 2000 - 1000);
    assert(bc.metrics.tokenBUserBalance.value.uiAmount == 500 - 100 - 100 + 50);
    assert(bc.metrics.tokenBProtocolBalance.value.uiAmount == 100 + 100 - 50);

    // --------------------------------------- depositing / windrawing from obligation\

    // check user has all LP tokens and protocol none
    assert(bc.metrics.tokenBLPUserBalance.value.uiAmount == 150);
    assert(bc.metrics.tokenBLPProtocolBalance.value.uiAmount == 0);
    // check obligation is initiated empty
    assert(bc.metrics.obligState.data.depositedValue.toNumber() == 0);
    assert(bc.metrics.obligState.data.allowedBorrowValue.toNumber() == 0);
    assert(bc.metrics.obligState.data.unhealthyBorrowValue.toNumber() == 0);

    // note: we're depositing LP tokens, so we need to make sure we've deposited enough liquidity above first
    await bc.depositObligationCollateral(bc.tokenB, 130);
    // note: need to refresh the oblig before printing metrics
    await bc.withdrawObligationCollateral(bc.tokenB, 30);
    await bc.refreshOblig();
    await bc.calcAndPrintMetrics();

    // check user deposited some of their LP tokens
    assert(bc.metrics.tokenBLPUserBalance.value.uiAmount == 50);
    assert(bc.metrics.tokenBLPProtocolBalance.value.uiAmount == 100);
    // check obligation no longer emptry (not checking for specific numbers due to price fluctuation)
    assert(bc.metrics.obligState.data.depositedValue.toNumber() > 0);
    assert(bc.metrics.obligState.data.allowedBorrowValue.toNumber() > 0);
    assert(bc.metrics.obligState.data.unhealthyBorrowValue.toNumber() > 0);

    // --------------------------------------- borrowing against collateral
    // the math:
    //  deposit 100 sol at $40 = $4000 collateral value
    //  LTV = 50%, means can borrow up to ~2k
    //  because also need to pay fees, can borrow up to ~1.9k
    //  using a value much smaller below because the price of SOL will likely move

    assert(bc.metrics.obligState.data.borrowedValue.toNumber() == 0);
    assert(bc.metrics.tokenAUserBalance.value.uiAmount == 5000);
    assert(bc.metrics.tokenAProtocolFeeBalance.value.uiAmount == 0);
    assert(bc.metrics.tokenAHostBalance.value.uiAmount == 0);

    await bc.borrowObligationLiquidity(bc.tokenA, bc.tokenB, 300);
    await bc.repayObligationLiquidity(bc.tokenA, bc.tokenB, 200);
    await bc.refreshOblig();
    await bc.calcAndPrintMetrics();

    // check obligation registers >0 as borrowed
    assert(bc.metrics.obligState.data.borrowedValue.toNumber() > 0);
    // check user got an extra 100 tokens from borrowing
    assert(bc.metrics.tokenAUserBalance.value.uiAmount == 5100);
    // check protocol and host both earned fees from the transaction
    assert(bc.metrics.tokenAProtocolFeeBalance.value.uiAmount > 0);
    assert(bc.metrics.tokenAHostBalance.value.uiAmount > 0);

    // --------------------------------------- liquidating collateral

    // note: only works when loan is below liquidation threshold, which makes it impossible to test
    //  it is possible to manually test this if we uncomment the if case in the liquidation instruction
    //  that is resposnible for ensuring liquidation value > LTV, then setting it to be < LTV
    // await bc.liquidateObligation(bc.tokenA, bc.tokenB, 99999);
    // await bc.refreshOblig();
    // await bc.calcAndPrintMetrics();

    // --------------------------------------- flash loan

    const oldBorrowedAmount = bc.metrics.obligState.data.borrowedValue.toNumber();
    const oldProtocolFee = bc.metrics.tokenAProtocolFeeBalance.value.uiAmount;
    const oldHostFee = bc.metrics.tokenAHostBalance.value.uiAmount;

    await bc.borrowFlashLoan(bc.tokenA, 100);
    await bc.calcAndPrintMetrics();

    //check that fees went up, but the borrowed amount stayed the same
    assert(bc.metrics.obligState.data.borrowedValue.toNumber() == oldBorrowedAmount);
    assert(bc.metrics.tokenAProtocolFeeBalance.value.uiAmount > oldProtocolFee);
    assert(bc.metrics.tokenAHostBalance.value.uiAmount > oldHostFee);

    console.log('All tests passed!');
}

main()
    .catch(err => {
        console.error(err);
        process.exit(-1);
    })
    .then(() => process.exit());
