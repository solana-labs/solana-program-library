import { PoolTokens } from "@/components/PoolTokens";
import { PositionColumn } from "@/components/Positions/PositionColumn";
import { PositionAccount } from "@/lib/PositionAccount";
import { useGlobalStore } from "@/stores/store";

interface Props {
  className?: string;
  positions: PositionAccount[];
}

export default function PoolPositionHeader(props: Props) {
  const allTokens = props.positions.map((position) => {
    return position.token;
  });

  const tokens = Array.from(new Set(allTokens));

  const poolData = useGlobalStore((state) => state.poolData);

  if (!props.positions[0]) return <p>No Positions</p>;

  return (
    <>
      <PositionColumn num={1}>
        <div className="flex max-w-fit items-center rounded-t bg-zinc-800 py-1.5 px-2">
          <PoolTokens tokens={tokens} />
          <div className="ml-1 text-sm font-medium text-white">
            {poolData[props.positions[0].pool.toString()]?.name}
          </div>
        </div>
      </PositionColumn>
      <PositionColumn num={2}>Leverage</PositionColumn>
      <PositionColumn num={3}>Net Value</PositionColumn>
      <PositionColumn num={4}>Collateral</PositionColumn>
      <PositionColumn num={5}>Entry Price</PositionColumn>
      <PositionColumn num={6}>Mark Price</PositionColumn>
      <PositionColumn num={7}>Liq. Price</PositionColumn>
    </>
  );
}
