import { getTokenIcon, TokenE } from "@/lib/Token";
import { Side } from "@/lib/types";
import {
  formatFees,
  formatNumber,
  formatNumberLessThan,
  formatPrice,
} from "@/utils/formatters";
import { cloneElement } from "react";
import { twMerge } from "tailwind-merge";

interface Props {
  className?: string;
  collateralToken: TokenE;
  positionToken: TokenE;
  entryPrice: number;
  liquidationPrice: number;
  fees: number;
  availableLiquidity: number;
  borrowRate: number;
  side: Side;
  onSubmit?(): void;
}

export function TradeDetails(props: Props) {
  const icon = getTokenIcon(props.positionToken);

  return (
    <div className={props.className}>
      <header className="mb-4 flex items-center">
        <div className="text-sm font-medium text-white">{props.side}</div>
        {cloneElement(icon, {
          className: twMerge(icon.props.className, "h-4", "ml-1.5", "w-4"),
        })}
        <div className="ml-0.5 text-sm font-semibold text-white">
          {props.positionToken}
        </div>
      </header>
      <div className={twMerge("grid", "grid-cols-2", "gap-4")}>
        {[
          {
            label: "Collateral in",
            value: "USD",
          },
          {
            label: "Entry Price",
            value: `$${formatNumber(props.entryPrice)}`,
          },
          {
            label: "Liq. Price",
            value: `$${formatNumber(props.liquidationPrice)}`,
          },
          {
            label: "Fees",
            value: `${formatNumberLessThan(props.fees)}`,
          },
          {
            label: "Borrow Rate",
            value: (
              <>
                {`${formatFees(100 * props.borrowRate)}% / hr`}
                <span className="text-zinc-500"> </span>
              </>
            ),
          },
          {
            label: "Available Liquidity",
            value: `$${formatPrice(props.availableLiquidity)}`,
          },
        ].map(({ label, value }, i) => (
          <div
            className={twMerge(
              "border-zinc-700",
              i < 4 && "pb-4",
              i < 4 && "border-b"
            )}
            key={i}
          >
            <div className="text-sm text-zinc-400">{label}</div>
            <div className="text-sm text-white">{value}</div>
          </div>
        ))}
      </div>
    </div>
  );
}
