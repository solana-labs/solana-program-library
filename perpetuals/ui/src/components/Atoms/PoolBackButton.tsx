import { ChevronLeft } from "@carbon/icons-react";
import { useRouter } from "next/router";
import { twMerge } from "tailwind-merge";

interface Props {
  className?: string;
}
export default function PoolBackButton(props: Props) {
  const router = useRouter();

  return (
    <div
      className={twMerge(
        "flex",
        "cursor-pointer",
        "items-center",
        "space-x-1.5",
        props.className
      )}
      onClick={() => router.push("/pools")}
    >
      <ChevronLeft className="h-4 w-4 fill-zinc-500" />

      <p className="text-sm font-medium text-zinc-500">Back To Pools</p>
    </div>
  );
}
