import Link from "next/link";
import { useRouter } from "next/router";
import { cloneElement } from "react";
import { twMerge } from "tailwind-merge";

interface Props extends React.AnchorHTMLAttributes<HTMLAnchorElement> {
  href: string;
  icon: JSX.Element;
}

export function NavbarLink(props: Props) {
  const router = useRouter();

  const currentPath = router.pathname;
  const selected = currentPath.startsWith(props.href);

  return (
    <Link
      href={props.href}
      className={twMerge(
        "font-medium",
        "flex",
        "h-full",
        "items-center",
        "px-5",
        "text-sm",
        "text-gray-500",
        "transition-colors",
        "active:text-gray-200",
        "hover:text-white",
        selected && "text-white",
        selected && "border-b",
        selected && "border-purple-500",
        props.className
      )}
    >
      <div className="hidden md:block">{props.children}</div>
      {cloneElement(props.icon, {
        className: twMerge(
          props.icon.props.className,
          "block",
          "fill-current",
          "h-4",
          "w-4",
          "md:hidden"
        ),
      })}
    </Link>
  );
}
