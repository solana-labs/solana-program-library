import {
  CheckCircleIcon,
  HamburgerIcon,
  Icon,
  SpinnerIcon,
  TimeIcon,
} from "@chakra-ui/icons";
import React, { useState } from "react";
import { FaThumbsUp, FaThumbsDown } from "react-icons/fa";
import {
  BsDashCircle,
  BsLightbulb,
  BsWallet,
  BsChevronDown,
  BsPencilSquare,
  BsFillExclamationCircleFill,
} from "react-icons/bs";
import Button from "../common/Button";
import { walletShortener } from "@/utils/util";
import {
  AvatarGroup,
  Avatar,
  FormControl,
  Input,
  Select,
} from "@chakra-ui/react";
import { useRouter } from "next/router";

export default function Proposal() {
  const [active, setActive] = useState(false);
  const route = useRouter();

  const toggle = () => {
    setActive(!active);
  };

  const send = () => {
    route.push("/proposal-summary");
  };
  const ActiveStyle = active
    ? " cursor-pointer bg-white text-blac focus-within:bg-primary"
    : "cursor-pointer bg-none rounded border-neutral-500 ";
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
          <Icon as={BsPencilSquare} className="inline-block mr-2" />
          Proposal rules
        </small>
        <hr className="my-2  border-neutral-600" />
        <div className="">
          <p className="font-bold text-neutral-100">
            Which wallet&apos;s rules should this proposal follow?
          </p>
          <small className="text-neutral-600">
            These rules determine voting duration, voting threshold, and vote
            tipping.
          </small>
          <div className="bg-neutral-800 rounded flex items-center justify-between p-2">
            <div className="flex items-center text-neutral-500 ">
              <Icon as={BsWallet} className="neutral-600" />
              <div className="ml-2">
                <small>{walletShortener()}</small>
                <p className="text-neutral-100 font-bold">$3230.23</p>
              </div>
            </div>
            <div className="flex items-center">
              <ul className="flex text-black">
                <AvatarGroup size="xs" max={3}>
                  <Avatar
                    name="Segun Adebayo"
                    src="https://cryptologos.cc/logos/usd-coin-usdc-logo.svg?v=025"
                  />
                  <Avatar
                    name="Ryan Florence"
                    src="https://cryptologos.cc/logos/solana-sol-logo.svg?v=025"
                  />

                  <Avatar
                    name="USDT"
                    src="https://cryptologos.cc/logos/tether-usdt-logo.svg?v=025"
                  />
                  <Avatar
                    name="Christian Nwamba"
                    src="https://bit.ly/code-beast"
                  />
                  <Avatar name="USDC" src="https://bit.ly/kent-c-dodds" />
                </AvatarGroup>
              </ul>
              <Icon className="ml-2" as={BsChevronDown} />
            </div>
          </div>
          <div className="my-2">
            <ul className="flex gap-1 justify-end">
              <li className="p-1 bg-neutral-800 text-neutral-500">
                {" "}
                <small>
                  {" "}
                  <TimeIcon /> 3D
                </small>{" "}
              </li>
              <li className="p-1 bg-neutral-800 text-neutral-500">
                {" "}
                <small>
                  {" "}
                  <SpinnerIcon /> 100,000
                </small>{" "}
              </li>
              <li className="p-1 bg-neutral-800 text-neutral-500">
                {" "}
                <small>
                  {" "}
                  <Icon as={CheckCircleIcon} /> 60%
                </small>{" "}
              </li>
              <li className="p-1 bg-neutral-800 text-neutral-500">
                {" "}
                <small>
                  {" "}
                  <Icon as={BsDashCircle} /> Strict
                </small>{" "}
              </li>
            </ul>
          </div>
        </div>
      </div>
      <div className="bg-black p-4 mt-4 rounded">
        <small className="capitalize text-neutral-500">
          <Icon as={BsLightbulb} className="inline-block mr-2" />
          Proposal Types
        </small>
        <hr className="my-2  border-neutral-600" />
        <div className="">
          <small className="bg-primary text-neutral-900 p-1 rounded">
            <HamburgerIcon /> New: Multiple choice
          </small>
          <p className="text-neutral-300 whitespace-normal  text-sm my-2">
            Now in addition to{" "}
            <span className="text-primary w-fit">
              <Icon as={FaThumbsUp} /> Yes <Icon as={FaThumbsDown} /> No
            </span>{" "}
            proposals, you can create proposals with multiple choices. This is
            great for polls, or for proposals that require multiple options to
            be selected.
          </p>
          <div className="">
            <p className="text-neutral-100 font-bold">
              What type of proposal are you creating?
            </p>
            <ul className="flex w-full my-2 text-center text-neutral-600 items-center gap-2 justify-between">
              <li
                onClick={toggle}
                className={`${ActiveStyle} px-4 py-2 rounded  w-1/2`}
              >
                <input type="radio" className="mx-1" />
                Executable (On-chain)
              </li>
              <li
                onClick={toggle}
                className={`${ActiveStyle} px-4 py-2 rounded  w-1/2`}
              >
                <input type="radio" className="mx-1" />
                Multiple choice poll (Off-chain)
              </li>
            </ul>
          </div>
        </div>
      </div>
      <div className="">
        <MultipleChoice />
        {/* <Executable /> */}
      </div>
      <div className="flex justify-between my-4">
        <Button text="X Close" rounded={true} variant="link" type="button" />
        <Button
          onClick={send}
          text="Continue &rarr;"
          variant="outline"
          type="button"
        />
      </div>
    </div>
  );
}

const MultipleChoice = () => {
  return (
    <div className="bg-black p-4 mt-4 rounded">
      <small className="capitalize text-neutral-500 ">
        <Icon as={BsLightbulb} className="inline-block mr-2" />
        Add Voting Choices
      </small>
      <hr className="my-2  border-neutral-600 mb-4" />
      <div className="flex flex-col gap-2">
        {[1, 2, 3].map((item, i) => (
          <VotingChoice key={i} />
        ))}
      </div>
      <div className="w-full my-4">
        <Button
          className="w-full rounded"
          text=" + Add another voting choice"
          type="button"
        />
      </div>
      <small className="text-neutral-500">
        <Icon as={BsFillExclamationCircleFill} /> {"  "}
        For all proposals, Realms auto-generates a voting option for “none of
        the above”, which will display below the last option added by the
        proposal creator.
      </small>
    </div>
  );
};

const Executable = () => {
  return (
    <div className="bg-black p-4 mt-4 rounded">
      <small className="capitalize text-neutral-500 ">
        <Icon as={BsLightbulb} className="inline-block mr-2" />
        Add Actions
      </small>
      <hr className="my-2  border-neutral-600 mb-4" />
      <small className="text-neutral-500 mb-6">
        Actions are the building blocks to creating an automatically executable
        smart contract in Realms. If a proposal is approved, its instructions
        will be executed based on the actions inputted..
      </small>{" "}
      <div className="flex flex-col gap-2 mt-2">
        {[1, 2, 3].map((item, i) => (
          <VotingAction key={i} />
        ))}
      </div>
      <Button
        className=" rounded-xl my-4 "
        colorScheme="white"
        text=" + Add Action"
        type="button"
        variant="outline"
      />
    </div>
  );
};

const VotingChoice = () => {
  return (
    <div className="bg-neutral-800 p-2 rounded-sm flex flex-col">
      <div className="flex justify-between">
        <h1 className="text-neutral-500">Choice 1</h1>
        <Button   variant="link" type="button" text="x Remove" />
      </div>
      <div className="">
        <p>
          <strong>Add a label</strong>
        </p>
        <small className="text-xs text-neutral-500">
          This is the text voters will see when they vote.
        </small>
      </div>
      <FormControl className="bg-transparent mt-4" isRequired>
        <Input className="" type="text" placeholder="Voting Choice " />
      </FormControl>
    </div>
  );
};

const VotingAction = () => {
  return (
    <div className="bg-neutral-800 p-2 rounded-sm flex flex-col">
      <h1 className="text-neutral-500">Action 1</h1>
      <div className="">
        <p>
          <strong>What action would you like to perform?</strong>
        </p>
      </div>
      <FormControl
        className="bg-transparent text-neutral-600   mt-4"
        isRequired
      >
        <Select
          className="placeholder:text-neutral-500"
          placeholder="Voting Choice "
        >
          <option value="value 1"> Transfer Tokens</option>
          <option value="value 2"> SNS Transfer Out Domain Name</option>
          <option value="value 3"> Action Type</option>
        </Select>
      </FormControl>
    </div>
  );
};
