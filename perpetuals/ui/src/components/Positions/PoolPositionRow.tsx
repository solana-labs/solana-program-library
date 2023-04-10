import { LoadingDots } from "@/components/LoadingDots";
import { PositionAdditionalInfo } from "@/components/Positions/PositionAdditionalInfo";
import PositionBasicInfo from "@/components/Positions/PositionBasicInfo";
import { PositionAccount } from "@/lib/PositionAccount";
import { useGlobalStore } from "@/stores/store";
import { getPerpetualProgramAndProvider } from "@/utils/constants";
import { ViewHelper } from "@/utils/viewHelpers";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { useEffect, useState } from "react";
import { twMerge } from "tailwind-merge";

interface Props {
  className?: string;
  position: PositionAccount;
}

export default function PoolPositionRow(props: Props) {
  const { connection } = useConnection();
  const { wallet } = useWallet();

  const poolData = useGlobalStore((state) => state.poolData);

  const [expanded, setExpanded] = useState(false);

  const [pnl, setPnl] = useState<number>(0);
  const [liqPrice, setLiqPrice] = useState(0);

  useEffect(() => {
    async function fetchData() {
      let { provider } = await getPerpetualProgramAndProvider(wallet as any);

      const View = new ViewHelper(connection, provider);

      let fetchedPnlPrice = await View.getPnl(props.position);

      let finalPnl = Number(fetchedPnlPrice.profit)
        ? Number(fetchedPnlPrice.profit)
        : -1 * Number(fetchedPnlPrice.loss);
      setPnl(finalPnl / 10 ** 6);

      let fetchedLiqPrice = await View.getLiquidationPrice(props.position);

      setLiqPrice(Number(fetchedLiqPrice) / 10 ** 6);
    }
    if (Object.keys(poolData).length > 0) {
      fetchData();
    }
  }, [poolData]);

  if (pnl === null) {
    return <LoadingDots />;
  }

  return (
    <div className={twMerge(expanded && "bg-zinc-800", props.className)}>
      <PositionBasicInfo
        className="transition-colors"
        expanded={expanded}
        position={props.position}
        pnl={pnl}
        liqPrice={liqPrice}
        onClickExpand={() => setExpanded((cur) => !cur)}
      />
      <PositionAdditionalInfo
        className={twMerge(
          "transition-all",
          expanded ? "opacity-100" : "opacity-0",
          expanded ? "py-5" : "py-0",
          expanded ? "h-auto" : "h-0"
        )}
        position={props.position}
        pnl={pnl}
        liqPrice={liqPrice}
      />
    </div>
  );
}
