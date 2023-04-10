import { twMerge } from "tailwind-merge";

interface Props {
  className?: string;
  children: [React.ReactNode, React.ReactNode, React.ReactNode];
}

export function PoolLayout(props: Props) {
  return (
    <div
      className={twMerge(
        "flex",
        "flex-col",
        "px-4",
        "lg:px-16",
        "mt-7",
        props.className
      )}
    >
      <div>{props.children[0]}</div>
      <div
        className={twMerge(
          "max-w-[1550px]",
          "w-full",
          "lg:gap-x-16",
          "lg:grid-cols-[1fr,424px]",
          "lg:grid"
        )}
      >
        <div>{props.children[1]}</div>
        <div>{props.children[2]}</div>
      </div>
    </div>
  );
}
