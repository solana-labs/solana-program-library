import {
  ChatIcon,
  EditIcon,
  HamburgerIcon,
  InfoIcon,
} from "@chakra-ui/icons";
import React from "react";

export default function Proposal() {
  return (
    <div className="w-fit p-2 mx-auto my-2 text-white">
      <div className="flex items-center gap-2 text-sm mb-2 ">
        <small className="bg-black p-2 text-white rounded">DAO Name</small>
        <small className="bg-neutral-800 text-neutral-500 p-2 rounded">
          Org type:{" "}
          <span className="text-neutral-50 ml-1">Community Token DAO</span>
        </small>
      </div>
      <h1 className="text-4xl font-bold text-white">
        Let&apos;s create a proposal
      </h1>

      <div className="bg-black p-4 mt-4 rounded">
        <small className="capitalize text-neutral-500">
          <EditIcon className="inline-block mr-2" />
          Proposal rules
        </small>
        <hr className="my-2  border-neutral-600" />
        <div className="">
          <p className="font-bold text-neutral-100">
            Which wallet&apos;s rules should this proposal follow?
          </p>
          <small className="text-neutral-600">
            These rules determin voting duration, voting threshold, and vote
            tipping.
          </small>
          <div className="bg-neutral-800 rounded flex items-center justify-between p-2">
            <div className="flex items-center text-neutral-500 ">
              <ChatIcon className="neutral-600" />
              <div className="ml-2">
                <small>Wallet Address</small>
                <p className="text-neutral-100 font-bold">$3230.23</p>
              </div>
            </div>
            <div className="">
              <p>Wallet address</p>
            </div>
          </div>
          <div className="my-2">
            <ul className="flex gap-1 justify-end">
              <li className="p-1 bg-neutral-800 text-neutral-500">
                {" "}
                <small>
                  {" "}
                  <ChatIcon /> 3D
                </small>{" "}
              </li>
              <li className="p-1 bg-neutral-800 text-neutral-500">
                {" "}
                <small>
                  {" "}
                  <ChatIcon /> 100,000
                </small>{" "}
              </li>
              <li className="p-1 bg-neutral-800 text-neutral-500">
                {" "}
                <small>
                  {" "}
                  <ChatIcon /> 60%
                </small>{" "}
              </li>
              <li className="p-1 bg-neutral-800 text-neutral-500">
                {" "}
                <small>
                  {" "}
                  <ChatIcon /> Strict
                </small>{" "}
              </li>
            </ul>
          </div>
        </div>
      </div>
      <div className="bg-black p-4 mt-4 rounded">
        <small className="capitalize text-neutral-500">
          <InfoIcon className="inline-block mr-2" />
          Proposal Types
        </small>
        <hr className="my-2  border-neutral-600" />
        <div className="">
          <small className="bg-primary text-neutral-900 p-1 rounded">
            <HamburgerIcon /> New: Multiple choice
          </small>
          <p className="text-neutral-300 text-sm my-2">
            Now in addition to Yes/No proposals, you can create proposals with
            multiple choices. This is great for polls, or for proposals that
            require multiple options to be selected.
          </p>
          <div className="">
            <p className="text-neutral-100 font-bold">
              What type of proposal are you creating?
            </p>
            <ul className="flex w-full my-2 text-center text-neutral-600 items-center gap-2 justify-between">
                <li className="px-4 py-2 border  border-neutral-500 rounded w-1/2 ">
                    
                    Executable (On-chain)
                </li>
                <li className="px-4 py-2 border rounded border-neutral-500 w-1/2 ">
                    Multiple choice poll (Off-chain)
                </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
