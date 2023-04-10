import { PositionRequest } from "@/hooks/storeHelpers/fetchPositions";
import { CustodyAccount } from "@/lib/CustodyAccount";
import { PoolAccount } from "@/lib/PoolAccount";
import { Custody, PriceStats } from "@/lib/types";
import { UserAccount } from "@/lib/UserAccount";
import { create } from "zustand";
import { devtools } from "zustand/middleware";

interface StoreState {
  positionData: PositionRequest;
  setPositionData: (position: PositionRequest) => void;

  poolData: Record<string, PoolAccount>;
  setPoolData: (pool: Record<string, PoolAccount>) => void;

  custodyData: Record<string, CustodyAccount>;
  setCustodyData: (custody: Record<string, CustodyAccount>) => void;

  userData: UserAccount;
  setUserData: (user: UserAccount) => void;

  priceStats: PriceStats;
  setPriceStats: (stats: PriceStats) => void;
}

export const useGlobalStore = create<StoreState>()(
  devtools((set, get) => ({
    devtools: false,

    positionData: {
      status: "pending",
    },
    setPositionData: (position: PositionRequest) =>
      set({ positionData: position }),

    poolData: {},
    setPoolData: (poolObjs: Record<string, PoolAccount>) =>
      set({ poolData: poolObjs }),

    custodyData: {},
    setCustodyData: (custody: Record<string, CustodyAccount>) =>
      set({ custodyData: custody }),

    userData: new UserAccount(),
    setUserData: (user: UserAccount) => set({ userData: user }),

    priceStats: {},
    setPriceStats: (stats: PriceStats) => set({ priceStats: stats }),
  }))
);
