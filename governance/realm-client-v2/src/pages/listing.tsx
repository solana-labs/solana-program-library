import NavbarMain from "@/components/common/Navbar_main";
import {
  AddIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  PhoneIcon,
  Search2Icon,
  SmallAddIcon,
} from "@chakra-ui/icons";
import {
  Avatar,
  Icon,
  IconButton,
  Input,
  InputGroup,
  InputLeftElement,
  Select,
  Switch,
  Tab,
  TabIndicator,
  TabList,
  TabPanel,
  TabPanels,
  Tabs,
} from "@chakra-ui/react";
import React from "react";
import { FiExternalLink, FiSettings, FiUsers } from "react-icons/fi";
import Image from "next/image";
import { walletShortener } from "@/utils/util";
import { BsPlusCircle } from "react-icons/bs";
import { useRouter } from "next/router";

export default function Listing() {
  return (
    <main>
      <NavbarMain />
      <div className="flex gap-2 p-4 text-white">
        <Listings />
        <ProfileDetails />
      </div>
    </main>
  );
}

const Listings = () => {
  return (
    <div className="bg-neutral-800 rounded flex flex-col gap-2 p-3 w-3/4">
      <p className="flex items-center ">
        <ChevronLeftIcon />
        Back
      </p>
      <div className="flex justify-between items-center">
        <div className="flex gap-2 items-center">
          <Avatar name="Creative Club" /> Creative Club
        </div>
        <ul className="flex gap-2 text-sm">
          <li className="cursor-pointer">
            <Icon as={FiUsers} /> Members (5)
          </li>
          <li className="cursor-pointer">
            {" "}
            <Icon as={FiSettings} /> Params
          </li>
          <li className="cursor-pointer">
            {" "}
            <Icon as={FiExternalLink} />
          </li>
        </ul>
      </div>
      <Tabs position="relative" variant="unstyled">
        <TabList>
          <Tab>Proposals</Tab>
          <Tab>About</Tab>
        </TabList>
        <TabIndicator
          mt="-1.5px"
          height="2px"
          bg="blue.500"
          borderRadius="1px"
        />

        <TabPanels>
          <TabPanel>
            <ProposalList />
          </TabPanel>
          <TabPanel>
            <p>About</p>
          </TabPanel>
        </TabPanels>
      </Tabs>
    </div>
  );
};

const ProposalList = () => {
  const router = useRouter()

  const toVoting = () => router.push("/voting")
  return (
    <div className="">
      <div className="flex gap-2">
        <InputGroup className="bg-black">
          <InputLeftElement pointerEvents="none">
            <Search2Icon color="gray.300" />
          </InputLeftElement>{" "}
          <Input className="w-3/4" placeholder="Search Proposal" size="md" />
        </InputGroup>
        <div className="flex gap-2">
          <Select className="w-8 text-neutral-500" placeholder="Filter">
            <option value="option1">Option 1</option>
            <option value="option2">Option 2</option>
            <option value="option3">Option 3</option>
          </Select>
          <Select className="w-20 text-neutral-500" placeholder="Sorting">
            <option value="option1">Option 1</option>
            <option value="option2">Option 2</option>
            <option value="option3">Option 3</option>
          </Select>
        </div>
      </div>
      <div className="flex justify-between  my-4">
        <p>19 Proposals</p>
        <ul className="flex gap-4 text-sm">
          <li className="text-neutral-500">
            Batch Voting <Switch size="sm" />
          </li>
          <li className="text-primary cursor-pointer">
            <Icon as={BsPlusCircle} /> New Proposal
          </li>
        </ul>
      </div>
      <ul className="flex flex-col gap-2">
        <li className="border border-neutral-600 rounded p-3">
          <div className="flex justify-between items-center mb-2">
            <p>Switch up the club’s configs</p>
            <div className="flex justify-between gap-2 items-center">
              <small className="text-primary text-xs p-2 rounded-full border border-primary">
                Finalizing
              </small>
              <ChevronRightIcon />
            </div>
          </div>
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
                  Almighty Realm{" "}
                  <span className="text-neutral-500">0 votes</span>
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
                  Adrian{" "}
                  <span className="text-neutral-500">1,413,114 votes</span>
                </small>
                <div className="w-[250px] h-1 my-1 bg-neutral-600 ">
                  <div className="w-[100px] h-1  bg-primary"></div>
                </div>
              </div>
              <div className="flex gap-1 items-center">
                <strong className="text-sm">17%</strong>
              </div>
            </li>
            <li className="p-2  border border-neutral-500 rounded-b-md flex text-neutral-500 justify-between">
              <div className="flex flex-col">
                <small>
                  3 more choices <ChevronRightIcon />
                </small>
              </div>
            </li>
          </ul>
        </li>
        <li className="border border-neutral-600 rounded p-3">
          <div className="flex justify-between items-center mb-2">
            <p>Send chicken NFTs</p>
            <div className="flex justify-between gap-2 items-center">
              <small className="text-primary text-xs p-2 rounded-full border border-primary">
                Finalizing
              </small>
              <IconButton  onClick={toVoting} aria-label='Back to voting' icon={<ChevronRightIcon />} />
            </div>
          </div>
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
                  Almighty Realm{" "}
                  <span className="text-neutral-500">0 votes</span>
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
                  Adrian{" "}
                  <span className="text-neutral-500">1,413,114 votes</span>
                </small>
                <div className="w-[250px] h-1 my-1 bg-neutral-600 ">
                  <div className="w-[100px] h-1  bg-primary"></div>
                </div>
              </div>
              <div className="flex gap-1 items-center">
                <strong className="text-sm">17%</strong>
              </div>
            </li>
            <li className="p-2  border border-neutral-500 rounded-b-md flex text-neutral-500 justify-between">
              <div className="flex flex-col">
                <small>
                  3 more choices <ChevronRightIcon />
                </small>
              </div>
            </li>
          </ul>
        </li>
      </ul>
    </div>
  );
};
const ProfileDetails = () => {
  return (
    <div className="w-1/  flex flex-col gap-2">
      <div className="p-3 bg-neutral-800 rounded pb-6 ">
        <div className="flex justify-between mb-2">
          <strong>My governance</strong>
          <small>
            View <ChevronRightIcon />
          </small>
        </div>
        <div className="flex items-center rounded justify-between gap-2 bg-black p-2">
          <div>
            <small className="text-xs whitespace-nowrap text-neutral-500">
              Creative club council votes
            </small>
            <p className="text-2xl">103</p>
          </div>
          <small className="text-neutral-400 text-xs">
            76.4% of total voting power
          </small>
        </div>
      </div>
      <div className="p-3 bg-neutral-800 rounded pb-6">
        <div className="flex justify-between mb-2">
          <strong>NFTs</strong>
          <small>
            View <ChevronRightIcon />
          </small>
        </div>
        <ul>
          <li>
            <Image
              src="https://bit.ly/ryan-florence"
              width={40}
              height={40}
              alt="nft name"
              className="rounded"
            />
          </li>
        </ul>
      </div>
      <div className="p-3 bg-neutral-800 rounded pb-6">
        <div className="flex justify-between mb-2">
          <strong>DAO Wallets & Assets</strong>
          <small>
            View <ChevronRightIcon />
          </small>
        </div>
        <div className="flex items-center rounded justify-between gap-2 bg-black p-2">
          <div>
            <small className="text-xs whitespace-nowrap text-neutral-500">
              Creative club council votes
            </small>
            <p className="text-2xl">$2</p>
          </div>
        </div>
        <ul className="my-2 flex flex-col gap-2">
          <li className="rounded border cursor-pointer border-neutral-600 flex gap-2 p-3 py-4 items-center h-20">
            <Avatar
              src="https://cryptologos.cc/logos/usd-coin-usdc-logo.svg?v=025"
              name="USDC"
              size={"xs"}
            />
            <div className="text-sm">
              <p className="m-0 font-bold">{walletShortener()}</p>
              <p className="text-sm text-neutral-500">2 USDC</p>
              <small className="text-sm text-neutral-500">=$2</small>
            </div>
          </li>
          <li className="rounded cursor-pointer border border-neutral-600 flex gap-2 p-3 py-4 items-center h-20">
            <Avatar
              src="https://cryptologos.cc/logos/solana-sol-logo.png?v=025"
              name="USDC"
              size={"xs"}
            />
            <div className="text-sm">
              <p className="m-0 font-bold">{walletShortener()}</p>
              <p className="text-sm text-neutral-500">0.2 SOL</p>
            </div>
          </li>
          <li className="rounded cursor-pointer border border-neutral-600 flex gap-2 p-3 py-4 items-center h-20">
            <Avatar
              src="https://cryptologos.cc/logos/tether-usdt-logo.svg?v=025"
              name="USDC"
              size={"xs"}
            />
            <div className="text-sm">
              <p className="m-0 font-bold">{walletShortener()}</p>
              <p className="text-sm text-neutral-500">5 USDT</p>
            </div>
          </li>
        </ul>
      </div>
    </div>
  );
};
