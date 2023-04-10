import { LoadingSpinner } from "@/components/Icons/LoadingSpinner";
import { LoadingDots } from "@/components/LoadingDots";
import { ExistingPositions } from "@/components/Positions/ExistingPositions";
import { NoPositions } from "@/components/Positions/NoPositions";
import { useGlobalStore } from "@/stores/store";
import { useWallet } from "@solana/wallet-adapter-react";

interface Props {
  className?: string;
}

export function Positions(props: Props) {
  const { publicKey } = useWallet();

  const positionData = useGlobalStore((state) => state.positionData);

  if (positionData.status === "pending") {
    return <LoadingSpinner className="text-4xl" />;
  }

  if (!publicKey) {
    return (
      <div className={props.className}>
        <header className="mb-5 flex items-center space-x-4">
          <div className="font-medium text-white">My Positions</div>
        </header>

        <NoPositions emptyString="No Open Positions" />
      </div>
    );
  }

  return (
    <div className={props.className}>
      <header className="mb-5 flex items-center space-x-4">
        <div className="font-medium text-white">My Positions</div>
        {positionData.status === "pending" && (
          <LoadingDots className="text-white" />
        )}
      </header>
      <ExistingPositions />
    </div>
  );
}
