import { LoadingSpinner } from "@/components/Icons/LoadingSpinner";
import { PoolAccount } from "@/lib/PoolAccount";
import { useGlobalStore } from "@/stores/store";
import { formatNumberCommas } from "@/utils/formatters";
import { getLiquidityBalance, getLiquidityShare } from "@/utils/retrieveData";
import { twMerge } from "tailwind-merge";

interface Props {
  pool: PoolAccount;
  className?: string;
}

export default function PoolGeneralStats(props: Props) {
  const stats = useGlobalStore((state) => state.priceStats);

  const userData = useGlobalStore((state) => state.userData);

  if (Object.keys(stats).length === 0 || props.pool.lpData === null) {
    return <LoadingSpinner className="absolute text-4xl" />;
  } else {
    return (
      <div
        className={twMerge(
          "grid",
          "grid-cols-4",
          "gap-x-4",
          "gap-y-8",
          props.className
        )}
      >
        {[
          {
            label: "Liquidity",
            value: `$${formatNumberCommas(props.pool.getLiquidities(stats))}`,
          },
          {
            label: "Volume",
            value: `$${formatNumberCommas(props.pool.getTradeVolumes())}`,
          },
          {
            label: "OI Long",
            value: (
              <>
                {`$${formatNumberCommas(props.pool.getOiLong())} `}
                <span className="text-zinc-500"> </span>
              </>
            ),
          },
          {
            label: "OI Short",
            value: `$${formatNumberCommas(props.pool.getOiShort())}`,
          },
          {
            label: "Fees",
            value: `$${formatNumberCommas(props.pool.getFees())}`,
          },
          {
            label: "Your Liquidity",
            value: `$${formatNumberCommas(
              getLiquidityBalance(
                props.pool,
                userData.getUserLpBalance(props.pool.address.toString()),
                stats
              )
            )}`,
          },
          {
            label: "Your Share",
            value: `${formatNumberCommas(
              Number(
                getLiquidityShare(
                  props.pool,
                  userData.getUserLpBalance(props.pool.address.toString())
                )
              )
            )}%`,
          },
        ].map(({ label, value }, i) => (
          <div
            className={twMerge("border-zinc-700", "border-t", "pt-3")}
            key={i}
          >
            <div className="text-sm text-zinc-400">{label}</div>
            <div className="text-sm text-white">{value}</div>
          </div>
        ))}
      </div>
    );
  }
}
