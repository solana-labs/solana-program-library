import ChevronDownIcon from "@carbon/icons-react/lib/ChevronDown";
import EditIcon from "@carbon/icons-react/lib/Edit";
import GrowthIcon from "@carbon/icons-react/lib/Growth";
import NewTab from "@carbon/icons-react/lib/NewTab";
import { cloneElement } from "react";
import { twMerge } from "tailwind-merge";

import { CollateralModal } from "@/components/Positions/CollateralModal";
import { PositionColumn } from "@/components/Positions/PositionColumn";
import { PositionAccount } from "@/lib/PositionAccount";
import { getTokenIcon, getTokenLabel } from "@/lib/Token";
import { Side } from "@/lib/types";
import { useGlobalStore } from "@/stores/store";
import { ACCOUNT_URL } from "@/utils/TransactionHandlers";
import { formatNumberCommas } from "@/utils/formatters";

interface Props {
  className?: string;
  expanded?: boolean;
  position: PositionAccount;
  pnl: number;
  liqPrice: number;
  onClickExpand?(): void;
}

export default function PositionBasicInfo(props: Props) {
  const tokenIcon = getTokenIcon(props.position.token);
  const stats = useGlobalStore((state) => state.priceStats);

  return (
    <div className={twMerge("flex", "items-center", "py-5", props.className)}>
      <PositionColumn num={1}>
        <div
          className={twMerge(
            "gap-x-2",
            "grid-cols-[32px,minmax(0,1fr)]",
            "grid",
            "items-center",
            "overflow-hidden",
            "pl-3"
          )}
        >
          {cloneElement(tokenIcon, {
            className: twMerge(
              tokenIcon.props.className,
              "flex-shrink-0",
              "h-8",
              "w-8"
            ),
          })}
          <div className="pr-2">
            <div className="font-bold text-white">{props.position.token}</div>
            <div className="mt-0.5 truncate text-sm font-medium text-zinc-500">
              {getTokenLabel(props.position.token)}
            </div>
          </div>
        </div>
      </PositionColumn>
      <PositionColumn num={2}>
        <div className="text-sm text-white">
          {props.position.getLeverage().toFixed(3)}x
        </div>
        <div
          className={twMerge(
            "flex",
            "items-center",
            "mt-1",
            "space-x-1",
            props.position.side === Side.Long
              ? "text-emerald-400"
              : "text-rose-400"
          )}
        >
          {props.position.side === Side.Long ? (
            <GrowthIcon className="h-3 w-3 fill-current" />
          ) : (
            <GrowthIcon className="h-3 w-3 -scale-y-100 fill-current" />
          )}
          <div className="text-sm">
            {props.position.side === Side.Long ? "Long" : "Short"}
          </div>
        </div>
      </PositionColumn>
      <PositionColumn num={3}>
        <div className="text-sm text-white">
          ${formatNumberCommas(props.position.getNetValue(props.pnl))}
        </div>
      </PositionColumn>
      <PositionColumn num={4}>
        <div className="flex items-center">
          <div className="text-sm text-white">
            ${formatNumberCommas(props.position.getCollateralUsd())}
          </div>
          <CollateralModal position={props.position} pnl={props.pnl}>
            <button className="group ml-2">
              <EditIcon
                className={twMerge(
                  "fill-zinc-500",
                  "h-4",
                  "transition-colors",
                  "w-4",
                  "group-hover:fill-white"
                )}
              />
            </button>
          </CollateralModal>
        </div>
      </PositionColumn>
      <PositionColumn num={5}>
        <div className="text-sm text-white">
          ${formatNumberCommas(props.position.getPrice())}
        </div>
      </PositionColumn>
      <PositionColumn num={6}>
        <div className="text-sm text-white">
          $
          {stats[props.position.token] != undefined
            ? formatNumberCommas(stats[props.position.token].currentPrice)
            : 0}
        </div>
      </PositionColumn>
      <PositionColumn num={7}>
        <div className="flex items-center justify-between pr-2">
          <div className="text-sm text-white">
            ${formatNumberCommas(props.liqPrice)}
          </div>
          <div className="flex items-center space-x-2">
            <a
              target="_blank"
              rel="noreferrer"
              href={`${ACCOUNT_URL(props.position.address.toString())}`}
            >
              <NewTab className="fill-white" />
            </a>
            <button
              className={twMerge(
                "bg-zinc-900",
                "grid",
                "h-6",
                "place-items-center",
                "rounded-full",
                "transition-all",
                "w-6",
                "hover:bg-zinc-700",
                props.expanded && "-rotate-180"
              )}
              onClick={() => props.onClickExpand?.()}
            >
              <ChevronDownIcon className="h-4 w-4 fill-white" />
            </button>
          </div>
        </div>
      </PositionColumn>
    </div>
  );
}
