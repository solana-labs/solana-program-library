import { LoadingSpinner } from "@/components/Icons/LoadingSpinner";
import { PoolTokens } from "@/components/PoolTokens";
import { PoolAccount } from "@/lib/PoolAccount";
import { useGlobalStore } from "@/stores/store";
import CheckmarkIcon from "@carbon/icons-react/lib/Checkmark";
import ChevronDownIcon from "@carbon/icons-react/lib/ChevronDown";
import * as Dropdown from "@radix-ui/react-dropdown-menu";
import { useState } from "react";
import { twMerge } from "tailwind-merge";

interface Props {
  className?: string;
  pool: PoolAccount;
  onSelectPool?(pool: PoolAccount): void;
}

export function PoolSelector(props: Props) {
  const [open, setOpen] = useState(false);

  const poolData = useGlobalStore((state) => state.poolData);

  if (!props.pool) {
    return <LoadingSpinner className="absolute text-4xl" />;
  }

  return (
    <Dropdown.Root open={open} onOpenChange={setOpen}>
      <Dropdown.Trigger
        className={twMerge(
          "bg-zinc-900",
          "gap-x-1",
          "grid-cols-[24px,1fr,24px]",
          "grid",
          "group",
          "h-11",
          "items-center",
          "px-4",
          "rounded",
          "text-left",
          "w-full",
          props.className
        )}
      >
        <PoolTokens tokens={props.pool.getTokenList()} className="h-5 w-5" />
        <div className="truncate text-sm font-medium text-white">
          {props.pool.name}
        </div>
        <div
          className={twMerge(
            "bg-zinc-900",
            "grid-cols-[24px,1fr,24px]",
            "grid",
            "h-8",
            "items-center",
            "px-4",
            "rounded",
            "text-left",
            "w-full"
          )}
        >
          <ChevronDownIcon className="fill-slate-500  transition-colors group-hover:fill-white" />
        </div>
      </Dropdown.Trigger>
      <Dropdown.Portal>
        <Dropdown.Content
          sideOffset={8}
          className="w-[392px] overflow-hidden rounded bg-zinc-900 shadow-2xl"
        >
          <Dropdown.Arrow className="fill-zinc-900" />
          {Object.values(poolData).map((pool) => (
            <Dropdown.Item
              className={twMerge(
                "cursor-pointer",
                "gap-x-1",
                "grid-cols-[24px,1fr,24px]",
                "grid",
                "group",
                "items-center",
                "px-4",
                "py-2.5",
                "text-left",
                "transition-colors",
                "w-full",
                "hover:bg-zinc-700"
              )}
              key={pool.address.toString()}
              onClick={() => props.onSelectPool?.(pool)}
            >
              <PoolTokens tokens={pool.getTokenList()} className="h-5 w-5" />
              <div>
                <div className="truncate text-sm font-medium text-white">
                  {pool.name}
                </div>
                <div className="text-xs text-zinc-500">
                  {pool.getTokenList().slice(0, 3).join(", ")}
                  {pool.getTokenList().length > 3
                    ? ` +${pool.getTokenList().length - 3} more`
                    : ""}
                </div>
              </div>
              {pool.address === props.pool.address ? (
                <CheckmarkIcon className="h-4 w-4 fill-white" />
              ) : (
                <div />
              )}
            </Dropdown.Item>
          ))}
        </Dropdown.Content>
      </Dropdown.Portal>
    </Dropdown.Root>
  );
}
