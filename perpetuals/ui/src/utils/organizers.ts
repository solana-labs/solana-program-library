import { PositionRequest } from "@/hooks/storeHelpers/fetchPositions";
import { PositionAccount } from "@/lib/PositionAccount";
import { PublicKey } from "@solana/web3.js";

export function getPoolSortedPositions(
  positionData: PositionRequest,
  user?: PublicKey
) {
  let sortedPositions: Record<string, PositionAccount[]> = {};

  if (
    positionData.status === "success" &&
    Object.values(positionData.data).length > 0
  ) {
    Object.values(positionData.data).forEach((position: PositionAccount) => {
      if (user && position.owner.toBase58() !== user.toBase58()) {
        return;
      }

      let pool = position.pool.toString();

      if (!sortedPositions[pool]) {
        sortedPositions[pool] = [];
      }

      sortedPositions[pool].push(position);
    });
  }

  return sortedPositions;
}

export function getUserPositionTokens(
  positionData: PositionRequest,
  user: PublicKey
): Record<string, number> {
  let positionTokens: Record<string, number> = {};

  if (
    positionData.status === "success" &&
    Object.values(positionData.data).length > 0 &&
    user
  ) {
    Object.values(positionData.data).forEach((position: PositionAccount) => {
      if (position.owner.toBase58() !== user.toBase58()) {
        return;
      }

      let tok = position.token;

      if (!positionTokens[tok]) {
        positionTokens[tok] = 1;
      }
    });
  }

  return positionTokens;
}

export function countDictList(dict: Record<string, any[]>) {
  return Object.values(dict).reduce((acc, val) => acc + val.length, 0);
}
