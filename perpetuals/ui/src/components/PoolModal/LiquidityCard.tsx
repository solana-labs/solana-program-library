import AirdropButton from "@/components/AirdropButton";
import { LpSelector } from "@/components/PoolModal/LpSelector";
import { SidebarTab } from "@/components/SidebarTab";
import { SolidButton } from "@/components/SolidButton";
import { TokenSelector } from "@/components/TokenSelector";
import { getCustodyData } from "@/hooks/storeHelpers/fetchCustodies";
import { getPoolData } from "@/hooks/storeHelpers/fetchPools";
import { PoolAccount } from "@/lib/PoolAccount";
import { TokenE } from "@/lib/Token";
import { Tab } from "@/lib/types";
import { useGlobalStore } from "@/stores/store";
import { getPerpetualProgramAndProvider } from "@/utils/constants";
import { ViewHelper } from "@/utils/viewHelpers";
import Add from "@carbon/icons-react/lib/Add";
import Subtract from "@carbon/icons-react/lib/Subtract";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { useEffect, useRef, useState } from "react";
import { changeLiquidity } from "src/actions/changeLiquidity";
import { twMerge } from "tailwind-merge";

interface Props {
  className?: string;
  pool: PoolAccount;
}

export default function LiquidityCard(props: Props) {
  const [tokenAmount, setTokenAmount] = useState(1);

  const [tab, setTab] = useState(Tab.Add);

  const [liqAmount, setLiqAmount] = useState(0);

  const { publicKey, wallet } = useWallet();
  const walletContextState = useWallet();

  const { connection } = useConnection();

  const [payToken, setPayToken] = useState(props.pool.getTokenList()[0]);
  const [fee, setFee] = useState<number>(0);

  const stats = useGlobalStore((state) => state.priceStats);

  const setPoolData = useGlobalStore((state) => state.setPoolData);
  const setCustodyData = useGlobalStore((state) => state.setCustodyData);

  const userData = useGlobalStore((state) => state.userData);

  // @ts-ignore
  let liqBalance = userData.lpBalances[props.pool.address.toString()];

  const [pendingRateConversion, setPendingRateConversion] = useState(false);

  const timeoutRef = useRef(null);

  async function changeLiq() {
    await changeLiquidity(
      walletContextState,
      connection,
      props.pool,
      props.pool.getCustodyAccount(payToken!),
      tokenAmount,
      liqAmount,
      tab
    );

    const custodyData = await getCustodyData();
    const poolData = await getPoolData(custodyData);

    setCustodyData(custodyData);
    setPoolData(poolData);
  }

  const [prevTokenAmount, setPrevTokenAmount] = useState(0);
  const [prevLiqAmount, setPrevLiqAmount] = useState(0);

  useEffect(() => {
    async function fetchData() {
      setPendingRateConversion(true);
      const { provider } = await getPerpetualProgramAndProvider(wallet as any);
      const View = new ViewHelper(connection, provider);
      let liqInfo;

      if (
        tab === Tab.Add &&
        tokenAmount !== 0 &&
        tokenAmount !== prevTokenAmount
      ) {
        liqInfo = await View.getAddLiquidityAmountAndFees(
          tokenAmount,
          props.pool!,
          props.pool!.getCustodyAccount(payToken!)!
        );

        setLiqAmount(Number(liqInfo.amount) / 10 ** props.pool.lpData.decimals);
        setPrevTokenAmount(tokenAmount);
      }

      if (
        tab === Tab.Remove &&
        liqAmount !== 0 &&
        liqAmount !== prevLiqAmount
      ) {
        liqInfo = await View.getRemoveLiquidityAmountAndFees(
          liqAmount,
          props.pool!,
          props.pool!.getCustodyAccount(payToken!)!
        );
        setTokenAmount(
          Number(liqInfo.amount) /
            10 ** props.pool.getCustodyAccount(payToken!)!.decimals
        );
        setPrevLiqAmount(liqAmount);
      }

      if (liqInfo) {
        setFee(Number(liqInfo.fee) / 10 ** 6);
      }

      setPendingRateConversion(false);
    }

    if (
      (tab === Tab.Add && tokenAmount == 0) ||
      (tab === Tab.Remove && liqAmount == 0)
    ) {
      setTokenAmount(0);
      setLiqAmount(0);
      setFee(0);
    }

    if (
      (tab === Tab.Add &&
        tokenAmount !== 0 &&
        tokenAmount !== prevTokenAmount) ||
      (tab === Tab.Remove && liqAmount !== 0 && liqAmount !== prevLiqAmount)
    ) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = setTimeout(fetchData, 1000);
    }

    return () => clearTimeout(timeoutRef.current);
  }, [tokenAmount, liqAmount]);

  const handleSelectToken = (token: TokenE) => {
    setTokenAmount(0);
    setPrevTokenAmount(0);
    setPrevLiqAmount(0);
    setPayToken(token);
  };

  return (
    <div className={props.className}>
      <div
        className={twMerge("bg-zinc-800", "p-4", "rounded", "overflow-hidden")}
      >
        <div className="mb-4 grid grid-cols-2 gap-x-1 rounded bg-black p-1">
          <SidebarTab
            selected={tab === Tab.Add}
            onClick={() => {
              setLiqAmount(0);
              setTokenAmount(0);
              setTab(Tab.Add);
            }}
          >
            <Add className="h-4 w-4" />
            <div>Add</div>
          </SidebarTab>
          <SidebarTab
            selected={tab === Tab.Remove}
            onClick={() => {
              setLiqAmount(0);
              setTokenAmount(0);
              setTab(Tab.Remove);
            }}
          >
            <Subtract className="h-4 w-4" />
            <div>Remove</div>
          </SidebarTab>
        </div>

        {props.pool.name == "TestPool1" &&
          Object.values(props.pool.custodies).map((custody) => {
            return (
              <AirdropButton
                key={custody.address.toString()}
                custody={custody}
              />
            );
          })}

        <div>
          <div className="flex items-center justify-between">
            {tab === Tab.Add ? (
              <>
                <div className="text-sm font-medium text-white">You Add</div>
                {publicKey && (
                  <div>
                    Balance:{" "}
                    {userData.tokenBalances[payToken] &&
                      userData.tokenBalances[payToken].toFixed(2)}
                  </div>
                )}
              </>
            ) : (
              <>
                <div className="text-sm font-medium text-white">You Remove</div>
                {publicKey && (
                  <div>Balance: {liqBalance && liqBalance.toFixed(2)}</div>
                )}
              </>
            )}
          </div>
          {tab === Tab.Add ? (
            <TokenSelector
              className="mt-2"
              amount={tokenAmount}
              token={payToken!}
              onChangeAmount={setTokenAmount}
              onSelectToken={handleSelectToken}
              tokenList={props.pool.getTokenList()}
              maxBalance={
                userData.tokenBalances[payToken]
                  ? userData.tokenBalances[payToken]
                  : 0
              }
            />
          ) : (
            <LpSelector
              className="mt-2"
              amount={liqAmount}
              onChangeAmount={setLiqAmount}
              maxBalance={liqBalance ? liqBalance : 0}
            />
          )}
        </div>
        <div className="mt-2">
          <div className="flex items-center justify-between">
            <div className="text-sm font-medium text-white">You Receive</div>
            {tab === Tab.Add ? (
              <>
                {publicKey && (
                  <div>Balance: {liqBalance && liqBalance.toFixed(2)}</div>
                )}
              </>
            ) : (
              <>
                {publicKey && (
                  <div>
                    Balance: {userData.tokenBalances[payToken].toFixed(2)}
                  </div>
                )}
              </>
            )}
          </div>

          {tab === Tab.Add ? (
            <LpSelector
              className="mt-2"
              amount={liqAmount}
              pendingRateConversion={pendingRateConversion}
            />
          ) : (
            // @ts-ignore
            <TokenSelector
              className="mt-2"
              amount={tokenAmount}
              token={payToken!}
              onSelectToken={handleSelectToken}
              tokenList={props.pool.getTokenList()}
              pendingRateConversion={pendingRateConversion}
            />
          )}
        </div>

        <div className="mt-2 flex flex-row justify-end space-x-2">
          <p className="text-sm text-white">${fee.toFixed(4)}</p>
          <p className="text-sm text-zinc-500">Fee</p>
        </div>
        <SolidButton
          className="mt-4 w-full"
          onClick={changeLiq}
          disabled={!publicKey || !tokenAmount}
        >
          {tab == Tab.Add ? "Add" : "Remove"} Liquidity
        </SolidButton>
        {!publicKey && (
          <p
            className="mt-2 text-center text-xs text-orange-500
      "
          >
            Please connect wallet to add liquidity
          </p>
        )}
        {!tokenAmount && (
          <p
            className="mt-2 text-center text-xs text-orange-500
      "
          >
            Please enter a valid amount of tokens to{" "}
            {tab === Tab.Add ? "add" : "remove"} liquidity
          </p>
        )}
      </div>
    </div>
  );
}
