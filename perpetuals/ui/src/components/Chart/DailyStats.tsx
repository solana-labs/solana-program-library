import { TokenE } from "@/lib/Token";
import { useGlobalStore } from "@/stores/store";
import { formatNumberCommas } from "@/utils/formatters";
import { twMerge } from "tailwind-merge";

interface DailyStatsProps {
  className?: string;
  token: TokenE;
}

export function DailyStats(props: DailyStatsProps) {
  const stats = useGlobalStore((state) => state.priceStats);

  if (Object.values(stats).length === 0) return <p>sdf</p>;

  return (
    <div
      className={twMerge("flex", "items-center", "space-x-5", props.className)}
    >
      <div>
        <div className="text-xs text-zinc-500">Current Price</div>
        <div className="text-sm text-white">
          ${formatNumberCommas(stats[props.token].currentPrice)}
        </div>
      </div>
      <div>
        <div className="text-xs text-zinc-500">24h Change</div>
        <div
          className={twMerge(
            "text-sm",
            stats[props.token].change24hr < 0 && "text-rose-400",
            stats[props.token].change24hr === 0 && "text-white",
            stats[props.token].change24hr > 0 && "text-emerald-400"
          )}
        >
          ${formatNumberCommas(stats[props.token].change24hr)}
        </div>
      </div>
    </div>
  );
}
