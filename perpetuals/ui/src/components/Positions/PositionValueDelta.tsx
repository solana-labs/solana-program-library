import {
  formatValueDelta,
  formatValueDeltaPercentage,
} from "@/utils/formatters";
import { twMerge } from "tailwind-merge";

interface Props {
  className?: string;
  valueDelta: number;
  valueDeltaPercentage: number;
}

export function PositionValueDelta(props: Props) {
  return (
    <div className={twMerge("flex", "items-center", props.className)}>
      <div
        className={twMerge(
          "text-sm",
          "font-medium",
          props.valueDelta > 0 ? "text-emerald-400" : "text-rose-400"
        )}
      >
        {props.valueDelta > 0 && "+"}
        {formatValueDelta(props.valueDelta)}
      </div>
      <div
        className={twMerge(
          "ml-1",
          "px-1",
          "rounded",
          "text-black",
          "text-sm",
          props.valueDeltaPercentage > 0 ? "bg-emerald-400" : "bg-rose-400"
        )}
      >
        {props.valueDeltaPercentage > 0 && "+"}
        {formatValueDeltaPercentage(props.valueDeltaPercentage)}%
      </div>
    </div>
  );
}
