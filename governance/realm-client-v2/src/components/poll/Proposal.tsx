import { ChatIcon, EditIcon } from "@chakra-ui/icons";
import React from "react";

export default function Proposal() {
  return (
    <div className="w-fit p-2 mx-auto my-2 text-white">
      <div className="flex items-center gap-2 text-sm ">
        <p className="bg-black p-2 text-white rounded">DAO Name</p>
        <p className="bg-neutral-800 text-neutral-500 p-2 rounded">
          Org type: <span className="text-neutral-50">Community Token DAO</span>
        </p>
      </div>
      <h1 className="text-4xl font-bold text-white">
        Let&apos;s create a proposal
      </h1>

      <div className="bg-black p-4 mt-4 rounded">
        <small className="capitalize text-neutral-500">
            <EditIcon className="inline-block mr-2" />
            Proposal rules</small>
        <hr className="my-2  border-neutral-600" />
        <div className="">
          <p className="font-bold text-neutral-100">
            Which wallet&apos;s rules should this proposal follow?
          </p>
          <small className="text-neutral-600">
            These rules determin voting duration, voting threshold, and vote
            tipping.
          </small>
          <div className="bg-neutral-800 flex items-center justify-between p-2">
            <div className="flex items-center text-neutral-500 ">
              <ChatIcon className="neutral-600" />
              <div className="ml-2">
                <small >Wallet Address</small>
                <p className="text-neutral-100 font-bold">$3230.23</p>
              </div>
            </div>
            <div className="">
              <p>Wallet address</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
