import CloseIcon from "@carbon/icons-react/lib/Close";
import * as Slider from "@radix-ui/react-slider";
import { twMerge } from "tailwind-merge";

function clamp(num: number, min: number, max: number) {
  return Math.min(max, Math.max(num, min));
}

interface Props {
  className?: string;
  value: number;
  minLeverage: number;
  maxLeverage: number;
  onChange?(value: number): void;
}

export function LeverageSlider(props: Props) {
  return (
    <div
      className={twMerge(
        "grid",
        "grid-cols-[max-content,max-content,1fr,max-content,max-content]",
        "items-center",
        props.className
      )}
    >
      <div className="text-xs text-zinc-400">Leverage</div>
      <div className="pl-6 pr-3 text-sm text-zinc-400">1x</div>
      <div>
        <Slider.Root
          min={props.minLeverage}
          max={props.maxLeverage}
          step={0.1}
          value={[props.value]}
          onValueChange={(values) => props.onChange?.(values[0] || 1)}
        >
          <Slider.Track className="relative block h-2 rounded-sm bg-zinc-900">
            <Slider.Range className="absolute block h-2 rounded-sm bg-purple-400" />
            <Slider.Thumb
              className={twMerge(
                "-translate-y-1/2",
                "bg-white",
                "block",
                "cursor-pointer",
                "h-5",
                "mt-1",
                "rounded-sm",
                "transition-all",
                "w-2",
                "hover:outline",
                "hover:outline-[3px]",
                "hover:outline-white/20"
              )}
            />
          </Slider.Track>
        </Slider.Root>
      </div>
      <div className="pl-3 pr-6 text-sm text-zinc-400">
        {props.maxLeverage}x
      </div>
      <div
        className={twMerge(
          "bg-zinc-900",
          "grid-cols-[1fr,max-content]",
          "grid",
          "items-center",
          "px-3",
          "py-2",
          "rounded",
          "w-20"
        )}
      >
        <input
          className="w-full bg-transparent text-center text-sm text-white"
          type="number"
          value={props.value}
          onChange={(e) => {
            const text = e.currentTarget.value;
            const number = parseFloat(text);
            props.onChange?.(
              Number.isNaN(number) ? 0 : clamp(number, 1, props.maxLeverage)
            );
          }}
          onBlur={(e) => {
            const text = e.currentTarget.value;
            const number = parseFloat(text);
            props.onChange?.(
              Number.isNaN(number) ? 1 : clamp(number, 1, props.maxLeverage)
            );
          }}
        />
        <button onClick={() => props.onChange?.(1)}>
          <CloseIcon
            className={twMerge(
              "fill-gray-500",
              "h-4",
              "transition-colors",
              "w-4",
              "hover:fill-white"
            )}
          />
        </button>
      </div>
    </div>
  );
}
