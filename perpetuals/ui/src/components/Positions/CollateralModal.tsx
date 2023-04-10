import { LpSelector } from "@/components/PoolModal/LpSelector";
import { SidebarTab } from "@/components/SidebarTab";
import { SolidButton } from "@/components/SolidButton";
import { TokenSelector } from "@/components/TokenSelector";
import { getPositionData } from "@/hooks/storeHelpers/fetchPositions";
import { PositionAccount } from "@/lib/PositionAccount";
import { Tab } from "@/lib/types";
import { useGlobalStore } from "@/stores/store";
import { getPerpetualProgramAndProvider } from "@/utils/constants";
import { formatNumberCommas } from "@/utils/formatters";
import { ViewHelper } from "@/utils/viewHelpers";
import Add from "@carbon/icons-react/lib/Add";
import ArrowRight from "@carbon/icons-react/lib/ArrowRight";
import Subtract from "@carbon/icons-react/lib/Subtract";
import * as Dialog from "@radix-ui/react-dialog";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { useEffect, useRef, useState } from "react";
import { changeCollateral } from "src/actions/changeCollateral";
import { twMerge } from "tailwind-merge";

interface Props {
  className?: string;
  children?: React.ReactNode;
  position: PositionAccount;
  pnl: number;
}

export function CollateralModal(props: Props) {
  const [tab, setTab] = useState(Tab.Add);
  const { publicKey } = useWallet();
  const walletContextState = useWallet();
  const { connection } = useConnection();

  const poolData = useGlobalStore((state) => state.poolData);
  const userData = useGlobalStore((state) => state.userData);

  let pool = poolData[props.position.pool.toString()]!;

  let payToken = props.position.token;

  let payTokenBalance = userData.tokenBalances[pool.getTokenList()[0]!];

  const custodyData = useGlobalStore((state) => state.custodyData);

  const setPositionData = useGlobalStore((state) => state.setPositionData);

  const [withdrawAmount, setWithdrawAmount] = useState(0);
  const [depositAmount, setDepositAmount] = useState(0);
  const [liqPrice, setLiqPrice] = useState(0);

  const [newCollateral, setNewCollateral] = useState(null);
  const [newLeverage, setNewLeverage] = useState(null);
  const [newLiqPrice, setNewLiqPrice] = useState(null);
  const [open, setOpen] = useState(false);

  const timeoutRef = useRef(null);

  useEffect(() => {
    async function fetchNewStats() {
      let { perpetual_program } = await getPerpetualProgramAndProvider(
        walletContextState
      );

      const View = new ViewHelper(
        perpetual_program.provider.connection,
        perpetual_program.provider
      );

      let fetchedOldLiq = await View.getLiquidationPrice(props.position);

      setLiqPrice(Math.round((fetchedOldLiq / 10 ** 6) * 100) / 100);

      if (tab === Tab.Add && depositAmount === 0) {
        setNewCollateral(null);
        setNewLeverage(null);
        setNewLiqPrice(null);
        return;
      }

      if (tab === Tab.Remove && withdrawAmount === 0) {
        setNewCollateral(null);
        setNewLeverage(null);
        setNewLiqPrice(null);
        return;
      }

      let liquidationPrice = await View.getLiquidationPrice(
        props.position,
        pool.getCustodyAccount(props.position.token)!,
        depositAmount,
        withdrawAmount
      );

      let newLiq = Math.round((liquidationPrice / 10 ** 6) * 100) / 100;

      setNewLiqPrice(newLiq);

      let newCollat;
      if (tab === Tab.Add) {
        newCollat =
          props.position.getCollateralUsd() +
          depositAmount * stats[props.position.token].currentPrice;
      } else {
        newCollat = props.position.getCollateralUsd() - withdrawAmount;
      }

      setNewCollateral(Math.round(newCollat * 100) / 100);

      let newLev;
      let changeCollateral =
        tab === Tab.Add
          ? depositAmount * stats[props.position.token].currentPrice
          : -1 * withdrawAmount;

      newLev =
        props.position.getSizeUsd() /
        (props.position.getCollateralUsd() + changeCollateral);

      setNewLeverage(Math.round(newLev * 100) / 100);
    }

    if (pool && props.position && payTokenBalance) {
      clearTimeout(timeoutRef.current);

      timeoutRef.current = setTimeout(() => {
        fetchNewStats();
      }, 1000);
    }
    return () => {
      clearTimeout(timeoutRef.current);
    };
  }, [open, withdrawAmount, depositAmount]);

  const stats = useGlobalStore((state) => state.priceStats);

  async function handleChangeCollateral() {
    let changeAmount;
    if (tab === Tab.Add) {
      changeAmount =
        depositAmount *
        10 ** pool.getCustodyAccount(props.position.token)!.decimals;
    } else {
      changeAmount = withdrawAmount * 10 ** 6;
    }

    await changeCollateral(
      walletContextState,
      connection,
      pool,
      props.position,
      tab === Tab.Add ? depositAmount : withdrawAmount,
      tab
    );

    const positionInfos = await getPositionData(custodyData);
    setPositionData(positionInfos);
  }

  return (
    <Dialog.Root
      open={open}
      onOpenChange={() => {
        setOpen(!open);
        setWithdrawAmount(0);
        setDepositAmount(0);
      }}
    >
      <Dialog.Trigger asChild>{props.children}</Dialog.Trigger>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed top-0 bottom-0 left-0 right-0 grid place-items-center bg-black/80 text-white">
          <Dialog.Content className="max-w-s mt-6 rounded bg-zinc-800 p-4">
            <div className="mb-2 grid grid-cols-2 gap-x-1 rounded bg-black p-1">
              <SidebarTab
                selected={tab === Tab.Add}
                onClick={() => {
                  setWithdrawAmount(0);
                  setDepositAmount(0);
                  setTab(Tab.Add);
                }}
              >
                <Add className="h-4 w-4" />
                <div>Deposit</div>
              </SidebarTab>
              <SidebarTab
                selected={tab === Tab.Remove}
                onClick={() => {
                  setWithdrawAmount(0);
                  setDepositAmount(0);
                  setTab(Tab.Remove);
                }}
              >
                <Subtract className="h-4 w-4" />
                <div>Withdraw</div>
              </SidebarTab>
            </div>
            <div>
              <div className="flex items-center justify-between">
                {tab === Tab.Add ? (
                  <>
                    <div className="text-sm font-medium text-white">
                      You Add
                    </div>
                    {publicKey && (
                      <div>
                        Max: {payTokenBalance && payTokenBalance.toFixed(3)}
                      </div>
                    )}
                  </>
                ) : (
                  <>
                    <div className="text-sm font-medium text-white">
                      You Remove
                    </div>
                    {publicKey && (
                      <div>
                        Max: {props.position.getCollateralUsd().toFixed(3)}
                      </div>
                    )}
                  </>
                )}
              </div>
              {tab === Tab.Add ? (
                <TokenSelector
                  className="mt-2"
                  amount={depositAmount}
                  token={payToken!}
                  onChangeAmount={setDepositAmount}
                  tokenList={[props.position.token]}
                  maxBalance={payTokenBalance}
                />
              ) : (
                <LpSelector
                  className="mt-2"
                  amount={withdrawAmount}
                  onChangeAmount={setWithdrawAmount}
                  maxBalance={props.position.getCollateralUsd()}
                  label={"USD"}
                />
              )}
            </div>

            <div className={twMerge("grid", "grid-cols-2", "gap-4", "pt-2")}>
              {[
                {
                  label: "Collateral",
                  value: `$${formatNumberCommas(
                    props.position.getCollateralUsd()
                  )}`,
                  newValue: `$${newCollateral}`,
                },
                {
                  label: "Mark Price",
                  value: `$${
                    stats[props.position.token] != undefined
                      ? formatNumberCommas(
                          stats[props.position.token].currentPrice
                        )
                      : 0
                  }`,
                },
                {
                  label: "Leverage",
                  value: `${props.position.getLeverage().toFixed(2)}`,
                  newValue: `${newLeverage}`,
                },
                {
                  label: "Size",
                  value: `$${formatNumberCommas(props.position.getSizeUsd())}`,
                },
                {
                  label: "Liq Price",
                  value: `$${liqPrice}`,
                  newValue: `$${newLiqPrice}`,
                },
              ].map(({ label, value, newValue }, i) => (
                <div
                  className={twMerge(
                    "border-zinc-700",
                    "pb-2",
                    i < 6 && "border-b",
                    i > 3 && "col-span-2"
                  )}
                  key={i}
                >
                  <div className="text-xs text-zinc-400">{label}</div>
                  <div className="space flex flex-row items-center space-x-1">
                    <div className="text-sm text-white">{value}</div>

                    {newValue &&
                      !(newValue === "null" || newValue === "$null") && (
                        <>
                          <p className="text-sm text-white">
                            <ArrowRight />
                          </p>

                          <div className="text-sm font-semibold text-white">
                            {newValue}
                          </div>
                        </>
                      )}
                  </div>
                </div>
              ))}
            </div>

            <div className="flex-end flex pt-2">
              <Dialog.Close asChild>
                <SolidButton
                  className="w-full"
                  disabled={!publicKey || (!depositAmount && !withdrawAmount)}
                  onClick={handleChangeCollateral}
                >
                  {tab === Tab.Add ? "Add Collateral" : "Remove Collateral"}
                </SolidButton>
              </Dialog.Close>
            </div>
          </Dialog.Content>
        </Dialog.Overlay>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
