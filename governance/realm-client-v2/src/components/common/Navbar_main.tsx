import React from "react";
import Logo from "./Logo";
import {
  Avatar,
  Icon,
} from "@chakra-ui/react";
import { ChevronDownIcon } from "@chakra-ui/icons";
import Link from "next/link";
import { FiExternalLink, FiSun, FiBell } from "react-icons/fi";
import { walletShortener } from "@/utils/util";

export default function NavbarMain() {
  return (
    <header className="sticky top-0 z-10 left-0 right-0 w-full p-4 text-neutral-400 flex justify-between">
      <Logo />
      <div className="flex items-center gap-3">
        <Link className="hover:text-white" href={"/"}>
          <p>
            Read the docs <Icon as={FiExternalLink} />
          </p>
        </Link>
        <p className="rounded-full w-10 h-10 p-1 hover:text-white cursor-pointer text-center bg-neutral-800">
          <Icon as={FiSun} />
        </p>
        <p className="rounded-full w-10 h-10 p-1 hover:text-white cursor-pointer text-center bg-neutral-800">
          <Icon as={FiBell} />
        </p>
        <div className="cursor-pointer h-10 flex gap-1 items-center bg-neutral-800 rounded-full p-2">
          <Avatar name="John Doe" size={"xs"} className="mr-1" />
          <div className="text-xs">
            <p className="text-primary ">
              <strong> @buckybuddyy</strong>
            </p>
            <small>{walletShortener()}</small>
          </div>
          <hr className="h-10 bg-neutral-600 w-0.5 m-1" />
          <ChevronDownIcon />
        </div>
      </div>
    </header>
  );
}
