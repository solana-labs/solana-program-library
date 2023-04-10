import ChartCandlestick from "@carbon/icons-react/lib/ChartCandlestick";

interface Props {
  className?: string;
  emptyString?: string;
}

export function NoPositions(props: Props) {
  return (
    <div className="flex flex-col items-center space-y-2 rounded-md bg-zinc-900 py-5">
      <ChartCandlestick className="h-5 w-5 fill-zinc-500" />
      <p className="text-sm font-normal text-zinc-500">{props.emptyString}</p>
    </div>
  );
}
