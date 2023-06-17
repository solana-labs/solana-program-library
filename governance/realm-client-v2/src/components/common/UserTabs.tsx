import {
  Tabs,
  TabList,
  TabPanels,
  Tab,
  TabPanel,
  TabIndicator,
  Icon,
  AvatarGroup,
  Avatar,
} from "@chakra-ui/react";
import React, { useState } from "react";
import {
  BiWorld,
  BiDotsHorizontalRounded,
  BiEditAlt,
  BiWalletAlt,
  BiNotepad,
  BiOutline,
  BiTimeFive,
  BiSpreadsheet,
  BiTimer,
} from "react-icons/bi";
import {
  BsLightbulb,
  BsRobot,
  BsFillExclamationCircleFill,
  BsChevronDown,
  BsArrowDownRight,
} from "react-icons/bs";
import {
  FaTwitter,
  FaDiscord,
  FaUsers,
  FaBalanceScaleLeft,
} from "react-icons/fa";
import { FiSettings } from "react-icons/fi";
import { Detail } from "../poll/Proposal_summary";
import Image from "next/image";
import {
  AiFillCheckCircle,
  AiOutlineMinusCircle,
  AiOutlineThunderbolt,
} from "react-icons/ai";

export default function UserTabs() {
  return (
    <div className=" w-full  text-white">
      <div className="bg-neutral-800 px-4 flex justify-between items-center">
        <Tabs position={"relative"} variant={"unstyled"}>
          <TabList className="text-neutral-500">
            <Tab className="flex gap-1">
              {" "}
              <Icon as={BiNotepad} /> FEED
            </Tab>
            <Tab className="flex gap-1">
              <Icon as={BiOutline} /> HUB
            </Tab>
            <Tab className="flex gap-1">
              <Icon as={BiWalletAlt} /> TREASURY
            </Tab>
          </TabList>
          <TabIndicator
            mt="-1.5px"
            height="2px"
            bg="blue.500"
            borderRadius="1px"
          />
        </Tabs>

        <div className="flex gap-4 items-center">
          <ul className="flex gap-2 text-neutral-500">
            <li>
              <Icon as={BiWorld} />
            </li>
            <li>
              <Icon as={FaTwitter} />
            </li>
            <li>
              <Icon as={FaDiscord} />
            </li>
            <li>
              <Icon as={BiDotsHorizontalRounded} />
            </li>
          </ul>
          <ul className="flex gap-2 text-neutral-500">
            <li className="p-1 w-8 h-8 text-center bg-black rounded">
              {" "}
              <Icon as={BiEditAlt} />
            </li>
            <li className="p-1 w-8 h-8 text-center bg-black rounded">
              {" "}
              <Icon as={FiSettings} />
            </li>
          </ul>
        </div>
      </div>
      <div className="flex justify-between m-10 gap-10">
        <PollDetails />
        <VoteDetails />
      </div>
    </div>
  );
}

const PollDetails = () => {
  return (
    <div className="w-2/3">
      <div className="">
        <div className="flex gap-2 my-3 items-center">
          <Image
            width={30}
            height={30}
            className="rounded-full"
            alt="User"
            src="https://bit.ly/dan-abramov"
          />
          <small>@nipsofposeidon</small>
          <small className="text-neutral-600">5 min ago</small>
        </div>
        <small className="p-1.5 rounded my-2 bg-black text-yellow-400">
          Multi-Option Poll
        </small>
        <h1 className="text-3xl my-2 font-bold">Poll: Who is best?</h1>
        <p className="flex border border-yellow-400 text-yellow-400 text-sm px-4 gap-1 py-2 items-center rounded">
          <Icon as={BsFillExclamationCircleFill} className="m-1" />
          <small>
            {" "}
            Note: Proposal titles and descriptions are manually inputted by the
            proposal creator. You may view the immutable details below
          </small>
        </p>
        <p className="text-neutral-400 text-sm my-3">
          This is placeholder text. Lorem ipsum represent a seismic shift in
          online culture. Breaking NFT records at launch, the brand quickly
          transcended from a virtuous Web3 community to a unique place in
          mainstream popular culture. <br /> <br /> It is the first Web3 brand
          to be signed by IMG, the world&apos;s leading brand licensing company
          representing global icons such as Angry Birds, Fortnite and Pepsi.
          Bruno Maglione ...
        </p>
        <p className="px-4 py-2 bg-neutral-700 w-fit rounded-sm  text-sm">
          + Read More
        </p>
      </div>
      <hr className="my-4  border-neutral-600" />
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
    </div>
  );
};

const VoteDetails = () => {
  const [voted, setVoted] = useState(false);
  return (
    <div className="w-1/3 flex flex-col gap-2">
      <div className="bg-neutral-800 p-4 w-[400px] rounded">
        <p className="uppercase text-xs whitespace-nowrap  my-2 ">
          <Icon as={FaUsers} /> Cast your community vote
        </p>
        {voted ? (
          <ul className="flex flex-col gap-2">
            <li className="p-2 text-center border border-primary text-primary rounded-full">
              Sebastian Dior
            </li>
            <li className="p-2 text-center border border-primary text-primary rounded-full">
              Dan Madden
            </li>
            <li className="p-2 text-center border border-primary text-primary rounded-full">
              Agriopa{" "}
            </li>
            <li className="p-2 text-center border border-primary text-primary rounded-full">
              Abstain{" "}
            </li>
          </ul>
        ) : (
          <ul className="flex flex-col gap-2">
            <li className="p-2 text-center bg-primary text-black rounded-full">
              <Icon as={AiFillCheckCircle} />
              Sebastian Dior
            </li>
            <li className="p-2 text-center text-neutral-500">Change Vote</li>
          </ul>
        )}
      </div>
      <div className="bg-neutral-800 p-4 w-[400px] rounded">
        <div className="flex justify-between items-center my-2">
          <div className="uppercase text-xs whitespace-nowrap  my-2 ">
            <Icon as={AiOutlineThunderbolt} /> My voting power
          </div>
          <div className="uppercase text-xs whitespace-nowrap  my-2 ">
            <div className=" text-sm text-neutral-600">
              <Icon as={BiTimer} /> Tvp
            </div>{" "}
            <strong>21%</strong>
          </div>
        </div>
        <ul className="flex flex-col gap-2">
          <li className="p-2 pl-4 py-3 bg-black rounded">
            <div className="flex flex-col">
              <small className="text-neutral-600">GRAPE Votes</small>
              <div className="text-2xl font-bold flex items-center gap-1">
                <Avatar
                  size={"xs"}
                  name="Grape Vote"
                  src="https://bit.ly/ryan-florence"
                />{" "}
                2,123.5260
              </div>
            </div>
          </li>
          <li className="text-sm text-neutral-600">
            You have 50.92453 GRAPE in your wallet.{" "}
          </li>
          <li className="cursor-pointer text-sm ">
            Deposit more <Icon as={BsArrowDownRight} />{" "}
          </li>
        </ul>
      </div>
      <div className="bg-neutral-800 p-4 w-[400px] rounded">
        <ul className="flex justify-between items-center my-2">
          <li className="uppercase text-xs whitespace-nowrap  my-2 ">
            <Icon as={BiTimeFive} /> Current Result
          </li>
          <li className=" text-xs bg-black px-2 py-1">
            <small>
              <Icon className="h-10" as={BiTimer} /> 7d 23h 59m
            </small>
            <div className="w-full my-1 rounded bg-neutral-700 h-1">
              <div className="w-2/3 bg-primary h-1"></div>
            </div>
          </li>
        </ul>
        <ul className="flex flex-col">
          <li className="p-2  border border-neutral-500 rounded-t-md flex justify-between">
            <div className="flex flex-col">
              <small>
                Sebastian Bor{" "}
                <span className="text-neutral-500">7,413,114 votes</span>
              </small>
              <div className="w-[250px] h-1 my-1 bg-neutral-600 ">
                <div className="w-[200px] h-1  bg-primary"></div>
              </div>
            </div>
            <div className="flex gap-1 items-center">
              <strong className="text-sm">73%</strong>
            </div>
          </li>
          <li className="p-2  border border-neutral-500 border-b border-t-0 flex justify-between">
            <div className="flex flex-col">
              <small>
                Almighty Realm <span className="text-neutral-500">0 votes</span>
              </small>
              <div className="w-[250px] h-1 my-1 bg-neutral-600 ">
                <div className="w-[1px] h-1  bg-primary"></div>
              </div>
            </div>
            <div className="flex gap-1 items-center">
              <strong className="text-sm">0%</strong>
            </div>
          </li>
          <li className="p-2  border border-neutral-500 border-y-0 flex justify-between">
            <div className="flex flex-col">
              <small>
                Adrian <span className="text-neutral-500">1,413,114 votes</span>
              </small>
              <div className="w-[250px] h-1 my-1 bg-neutral-600 ">
                <div className="w-[100px] h-1  bg-primary"></div>
              </div>
            </div>
            <div className="flex gap-1 items-center">
              <strong className="text-sm">17%</strong>
            </div>
          </li>
          <li className="p-2  border border-neutral-500 rounded-b-md flex justify-between">
            <div className="flex flex-col">
              <small>
                None of the Above{" "}
                <span className="text-neutral-500">7,413 votes</span>
              </small>
              <div className="w-[250px] h-1 my-1 bg-neutral-600 ">
                <div className="w-[75px] h-1  bg-primary"></div>
              </div>
            </div>
            <div className="flex gap-1 items-center">
              <strong className="text-sm">10%</strong>
            </div>
          </li>
        </ul>
        <div className="mt-3 text-neutral-600 flex gap-1 text-sm">
          <AvatarGroup size="xs" max={3}>
            <Avatar name="Ryan Florence" src="https://bit.ly/ryan-florence" />
            <Avatar name="Segun Adebayo" src="https://bit.ly/sage-adebayo" />
            <Avatar name="Kent Dodds" src="https://bit.ly/kent-c-dodds" />
          </AvatarGroup>
          7,437,805 total votes
        </div>
      </div>
      <div className="bg-neutral-800 p-4 w-[400px] rounded">
        <p className="uppercase text-xs whitespace-nowrap  my-2 ">
          <Icon as={BiSpreadsheet} /> Voting Rules
        </p>
        <div className="flex justify-between items-center">
          <ul className="flex gap-1 items-center">
            <li className="text-xs p-1 bg-black">
              <Icon as={FaUsers} /> Community
            </li>
            <li className="text-xs p-1 bg-black">
              <Icon as={BiTimeFive} /> 3D
            </li>
            <li className="text-xs p-1 bg-black">
              <Icon as={FaBalanceScaleLeft} /> 60%
            </li>
            <li className="text-xs p-1 bg-black">
              <Icon as={AiOutlineMinusCircle} /> Strict
            </li>
          </ul>
          <p className="">
            <Icon as={BsChevronDown} />
          </p>
        </div>
      </div>
    </div>
  );
};
