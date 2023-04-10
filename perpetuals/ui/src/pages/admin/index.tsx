import { LoadingSpinner } from "@/components/Icons/LoadingSpinner";
import { ExistingPositions } from "@/components/Positions/ExistingPositions";
import { useGlobalStore } from "@/stores/store";

interface Props {
  className?: string;
}

export default function Admin(props: Props) {
  const positionData = useGlobalStore((state) => state.positionData);
  return (
    <div className={props.className}>
      <header className="mb-5 flex items-center space-x-4">
        <div className="font-medium text-white">All Positions</div>
        {positionData.status === "pending" && (
          <LoadingSpinner className="text-4xl" />
        )}
      </header>
      <ExistingPositions />
    </div>
  );
}
