import { getTokenIcon, TokenE } from "@/lib/Token";
import { cloneElement } from "react";
import { twMerge } from "tailwind-merge";

interface Props {
  className?: string;
  tokens: TokenE[];
}

export function PoolTokens(props: Props) {
  return (
    <div className="flex items-center -space-x-6">
      {props.tokens.slice(0, 3).map((token, i) => {
        const tokenIcon = getTokenIcon(token);

        return cloneElement(tokenIcon, {
          className: twMerge(
            tokenIcon.props.className,
            props.className,
            "border-black",
            "border",
            "rounded-full",
            "relative",
            "shrink-0"
          ),
          style: { zIndex: 3 - i },
          key: i,
        });
      })}
    </div>
  );
}
