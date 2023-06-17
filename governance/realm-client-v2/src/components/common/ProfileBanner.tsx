import { BannerImageUrl } from "@/utils/util";
import { Avatar, Icon } from "@chakra-ui/react";
import Image from "next/image";
import React from "react";
import { AiOutlineDollarCircle } from "react-icons/ai";
import Button from "./Button";
import { FaUsers,FaImage } from "react-icons/fa";

export default function ProfileBanner() {
  return (
    <div className="sticky">
      <Image
        // sizes=" 100vw"
        src={BannerImageUrl}
        className="w-full h-36 object-cover"
        alt={"banner"}
        width={1000}
        height={100}
      />
      <div className="mx-auto">
        <div className="text-white bg-neutral-800  flex  items-center w-full justify-between p-4">
          <div className="flex">
            <div className="border-4 overflow-hidden  border-neutral-800  -mt-16 rounded-full w-[100px] z-3 ">
              <Image
                src="https://bit.ly/dan-abramov"
                alt="User"
                width={100}
                height={100}
              />
            </div>
            <p className="text-2xl">Okay Bear</p>
          </div>
          <ul className="flex items-center gap-2">
            <li>
              <small className="text-neutral-500">NFT Floor Price</small>
              <p className="flex items-center gap-1">
                <Image
                  src="https://bit.ly/dan-abramov"
                  alt="Okay Bear"
                  width={20}
                  height={20}
                  className="rounded"
                />{" "}
                <Icon as={AiOutlineDollarCircle} />
                64.69
              </p>
            </li>
            <li className="flex gap-2 border border-neutral-500 rounded p-2 cursor-pointer">
              <p className="flex gap-1 items-center">
                {" "}
                <Icon as={FaUsers} className="fill-primary"/>
                Community
              </p>
            </li>
            <li className="flex gap-2  rounded p-2 cursor-pointer bg-primary text-black font-bold">
              <p className="flex gap-1 items-center">
                {" "}
                <Icon as={FaImage} />
                Buy NFT
              </p>
            </li>
          </ul>
        </div>
      </div>
    </div>
  );
}
