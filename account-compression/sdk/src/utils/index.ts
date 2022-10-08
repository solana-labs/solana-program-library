import {
  PublicKey,
} from "@solana/web3.js";

export const SPL_NOOP_ADDRESS = "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV";
export const SPL_NOOP_PROGRAM_ID = new PublicKey(SPL_NOOP_ADDRESS);

type DepthSizePair = {
  maxDepth: number,
  maxBufferSize: number
};

const allPairs: number[][] = [
  [3, 8],
  [5, 8],
  [14, 64],
  [14, 256],
  [14, 1024],
  [14, 2048],
  [20, 64],
  [20, 256],
  [20, 1024],
  [20, 2048],
  [24, 64],
  [24, 256],
  [24, 512],
  [24, 1024],
  [24, 2048],
  [26, 512],
  [26, 1024],
  [26, 2048],
  [30, 512],
  [30, 1024],
  [30, 2048],
];

export const ALL_DEPTH_SIZE_PAIRS: DepthSizePair[] = allPairs.map((pair) => {
  return {
    maxDepth: pair[0],
    maxBufferSize: pair[1]
  }
})