import { LoadingSpinner } from "@/components/Icons/LoadingSpinner";
import { PoolAccount } from "@/lib/PoolAccount";
import { getTokenIcon, getTokenLabel } from "@/lib/Token";
import { getCurrentWeight } from "@/lib/classGetters";
import { useGlobalStore } from "@/stores/store";
import { ACCOUNT_URL } from "@/utils/TransactionHandlers";
import { formatNumberCommas } from "@/utils/formatters";
import NewTab from "@carbon/icons-react/lib/NewTab";
import { cloneElement } from "react";
import { twMerge } from "tailwind-merge";

interface Props {
  className?: string;
  pool: PoolAccount;
}

export default function PoolTokenStats(props: Props) {
  const stats = useGlobalStore((state) => state.priceStats);
  let poolData = useGlobalStore((state) => state.poolData);

  if (Object.keys(stats).length === 0) {
    return <LoadingSpinner className="absolute text-4xl" />;
  } else {
    return (
      <div className="w-full ">
        <div className="bg-zinc-900 p-8">
          <table className={twMerge("table-auto", "text-white", "w-full")}>
            <thead className={twMerge("text-xs", "text-zinc-500", "p-10")}>
              <tr className="">
                <td className="pb-5 text-white">Pool Tokens</td>
                <td className="pb-5">Deposit Fee</td>
                <td className="pb-5">Liquidity</td>
                <td className="pb-5">Price</td>
                <td className="pb-5">Amount</td>
                <td className="pb-5">Current/Target Weight</td>
                <td className="pb-5">Utilization</td>
                <td className="pb-5"></td>
              </tr>
            </thead>
            <tbody className={twMerge("text-xs")}>
              {Object.values(props.pool.custodies).map((custody) => {
                let pool = poolData[custody.pool.toString()];
                let token = custody.getTokenE();

                if (!token) return <></>;

                return (
                  <tr
                    key={custody.mint.toString()}
                    className="border-t border-zinc-700"
                  >
                    <td className="py-4">
                      <div className="flex flex-row items-center space-x-1">
                        {cloneElement(getTokenIcon(custody.getTokenE()!), {
                          className: "h-10 w-10",
                        })}
                        <div className="flex flex-col">
                          <p className="font-medium">{custody.getTokenE()!}</p>
                          <p className={twMerge("text-xs", "text-zinc-500")}>
                            {getTokenLabel(custody.getTokenE()!)}
                          </p>
                        </div>
                        <a
                          target="_blank"
                          rel="noreferrer"
                          href={`${ACCOUNT_URL(custody.mint.toString())}`}
                        >
                          <NewTab />
                        </a>
                      </div>
                    </td>
                    <td>{custody.getAddFee()}%</td>
                    <td>
                      ${formatNumberCommas(custody.getCustodyLiquidity(stats))}
                    </td>
                    <td>${formatNumberCommas(stats[token].currentPrice)}</td>
                    <td>{formatNumberCommas(custody.getAmount())}</td>
                    <td>
                      {formatNumberCommas(
                        getCurrentWeight(props.pool, custody, stats)
                      )}
                      % /{" "}
                      {Number(pool?.getRatioStruct(custody.address)!.target) /
                        100}
                      %
                    </td>
                    <td>{formatNumberCommas(custody.getUtilizationRate())}%</td>
                    <td>
                      <a
                        target="_blank"
                        rel="noreferrer"
                        href={`${ACCOUNT_URL(custody.address.toString())}`}
                      >
                        <NewTab />
                      </a>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    );
  }
}
