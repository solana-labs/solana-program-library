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
  BsRobot,
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
  FormLabel,
  Textarea,
} from "@chakra-ui/react";

export default function Index() {
  const [active, setActive] = useState(false);

  const toggle = () => {
    setActive(!active);
  };
  const ActiveStyle = active
    ? " cursor-pointer bg-white text-blac focus-within:bg-primary"
    : "cursor-pointer bg-none rounded border-neutral-500 ";
  return (
    <div className="w-fit p-2 mx-auto my-2 text-white">
      <div className="flex items-center gap-2 text-sm mb-2 ">
        <small className="bg-black p-2 text-white rounded">DAO Name Here</small>
        <small className="bg-neutral-800 text-neutral-500 p-2 rounded">
          Org type:{" "}
          <span className="text-neutral-50 ml-1">Community Token DAO</span>
        </small>
        <small className=" p-2 bg-neutral-800 text-white rounded">
          <Icon as={BsWallet} /> {walletShortener()}
        </small>
      </div>
      <h1 className="text-4xl font-bold text-white">
        Add a title and description for your proposal.{" "}
      </h1>
      <small className="text-neutral-500 my-2">
        Before submitting, ensure your description is correct and rules updates
        are accurate.
      </small>

      <div className="bg-black p-4 mt-4 rounded">
        <small className="capitalize text-neutral-500">
          <Icon as={BsLightbulb} className="inline-block mr-2" />
          Title & Description{" "}
        </small>
        <hr className="my-2  border-neutral-600" />
        <div className="mt-6 flex flex-col gap-3">
          <FormControl>
            <FormLabel>Proposal Title</FormLabel>
            <Input
              className="placeholde:text-neutral-300 bg-neutral-700"
              type="text"
              placeholder="e.g. Send USDC to wallet address"
            />
          </FormControl>
          <FormControl>
            <FormLabel>
              Proposal Description
              <br />
              <small className="text-neutral-500">
                This will help voters understand more details about your
                proposed changes.
              </small>
            </FormLabel>

            <Textarea
              className="placeholde:text-neutral-300"
              placeholder="e.g. Send USDC to wallet address"
            />
          </FormControl>
        </div>
      </div>
      <div className="my-4 mt-8">
        <h1>
          <strong>Proposal Details</strong>
        </h1>
        <p className="text-primary whitespace-normal flex items-center  text-sm my-2">
          <Icon as={BsRobot} className="mr-2" /> This section is automatically
          generated
        </p>
      </div>
      <div className="bg-black p-4 mt-4 rounded">
        <small className="capitalize text-neutral-500">
          <Icon as={BsLightbulb} className="inline-block mr-2" />
          Rules
        </small>
        <hr className="my-2  border-neutral-600" />
        <div className="my-8">
          <ul className="grid grid-cols-2 row-auto gap-3">
            <div className="col-span-2">
              <Detail small="Wallet Address" content={walletShortener()} />
            </div>
            <Detail small="Vote Type" content="Community" />
            <Detail small="Approval Quorum" content="60%" />
            <Detail small="Vote Tipping" content="Strict" />
            <Detail small="Veto Power" content="Council" />
            <Detail small="Veto Quorum" content="3%" />
            <Detail small="Cool-Off Voting Time" content="22 hours" />
          </ul>
        </div>
      </div>
      <div className="my-2 bg-black p-4">
        <ul className="grid grid-cols-2 row-auto gap-3">
          <div className="col-span-2">
            <Detail small="Total Voting Duration" content="3 days" />
          </div>{" "}
          <Detail small="Unrestricted Voting Time" content="3 Days" />
          <Detail small="Cool-Off Voting Time" content="12 hours" />
        </ul>
      </div>
      <div className="bg-black p-4 mt-4 rounded">
        <small className="capitalize text-neutral-500">
          <Icon as={BsLightbulb} className="inline-block mr-2" />
          Voting Options
        </small>
        <hr className="my-2  border-neutral-600" />
        <div className="my-8">
          <ul className="flex flex-col gap-3">
            <Detail small="Option 1" content="Sebastian Dior" />
            <Detail small="Option 2" content="Dan Madden" />
            <Detail small="Option 3" content="Agriopa" />
            <Detail small="Option 4" content="Abstain" />
          </ul>
        </div>
      </div>
      <div className="flex gap-2 justify-end my-4">
        <Button
          text="Edit Proposal"
          rounded={true}
          variant="link"
          type="button"
        />
        <Button
          className="rounded-md"
          text={`Create Proposal `}
          variant="solid"
          type="button"
        />
      </div>
    </div>
  );
}

interface DetailProps {
  small: string;
  content: string;
}
export const Detail = (props: DetailProps) => {
  return (
    <li className="border-l  border-neutral-500 pl-4">
      <div className="">
        <small className="text-neutral-400"> {props.small}</small>
        <p>{props.content}</p>
      </div>
    </li>
  );
};
