import { SolanaIconCircle } from "@/components/Icons/SolanaIconCircle";
import { UsdcIconCircle } from "@/components/Icons/UsdcIconCircle";
import { MSolIconCircle } from "@/components/Icons/MSolIconCircle";
import { STSolIconCircle } from "@/components/Icons/STSolIconCircle";
import { RayIconCircle } from "@/components/Icons/RayIconCircle";
import { UsdtIconCircle } from "@/components/Icons/UsdtIconCircle";
import { OrcaIconCircle } from "@/components/Icons/OrcaIconCircle";
import { BonkIconCircle } from "@/components/Icons/BonkIconCircle";

export enum TokenE {
  SOL = "SOL",
  mSOL = "mSOL",
  stSOL = "stSOL",
  USDC = "USDC",
  USDT = "USDT",
  RAY = "RAY",
  ORCA = "ORCA",
  Bonk = "Bonk",
  TEST = "Test",
}
export const TOKEN_LIST = [
  TokenE.SOL,
  TokenE.mSOL,
  TokenE.stSOL,
  TokenE.USDC,
  TokenE.USDT,
  TokenE.RAY,
  TokenE.ORCA,
  TokenE.Bonk,
  TokenE.TEST,
];

export function asToken(tokenStr: string): TokenE {
  switch (tokenStr) {
    case "SOL":
      return TokenE.SOL;

    case "mSOL":
      return TokenE.mSOL;
    case "stSOL":
      return TokenE.stSOL;
    case "USDC":
      return TokenE.USDC;
    case "USDT":
      return TokenE.USDT;
    case "RAY":
      return TokenE.RAY;
    case "ORCA":
      return TokenE.ORCA;
    case "Bonk":
      return TokenE.Bonk;
    case "Test":
      return TokenE.TEST;
    default:
      throw new Error("Not a valid token string");
  }
}

export function getTokenLabel(token: TokenE) {
  switch (token) {
    case TokenE.SOL:
      return "Solana";
    case TokenE.USDC:
      return "UDC Coin";
    case TokenE.mSOL:
      return "Marinade Staked SOL";
    case TokenE.stSOL:
      return "Lido Staked SOL";
    case TokenE.RAY:
      return "Raydium";
    case TokenE.USDT:
      return "USDT";
    case TokenE.ORCA:
      return "Orca";
    case TokenE.Bonk:
      return "BonkCoin";
    case TokenE.TEST:
      return "Test Token";
  }
}

export function getSymbol(token: TokenE) {
  switch (token) {
    case TokenE.Bonk:
      return "BONKUSDT";
    case TokenE.ORCA:
      return "ORCAUSD";
    case TokenE.RAY:
      return "RAYUSD";
    case TokenE.SOL:
      return "SOLUSD";
    case TokenE.USDC:
      return "USDCUSD";
    case TokenE.USDT:
      return "USDTUSD";
    case TokenE.mSOL:
      return "MSOLUSD";
    case TokenE.stSOL:
      return "STSOLUSDT";
    case TokenE.TEST:
      return "LTCUSD";
  }
}

export function getTokenIcon(token: TokenE) {
  switch (token) {
    case TokenE.SOL:
      return <SolanaIconCircle />;
    case TokenE.USDC:
      return <UsdcIconCircle />;
    case TokenE.mSOL:
      return <MSolIconCircle />;
    case TokenE.stSOL:
      return <STSolIconCircle />;
    case TokenE.RAY:
      return <RayIconCircle />;
    case TokenE.USDT:
      return <UsdtIconCircle />;
    case TokenE.ORCA:
      return <OrcaIconCircle />;
    case TokenE.Bonk:
      return <BonkIconCircle />;
    case TokenE.TEST:
      return <BonkIconCircle />;
  }
}

export function getTokenId(token: TokenE) {
  switch (token) {
    case TokenE.SOL:
      return "solana";
    case TokenE.mSOL:
      return "msol";
    case TokenE.stSOL:
      return "lido-staked-sol";
    case TokenE.USDC:
      return "usd-coin";
    case TokenE.USDT:
      return "tether";
    case TokenE.RAY:
      return "raydium";
    case TokenE.ORCA:
      return "orca";
    case TokenE.Bonk:
      return "bonk";
    case TokenE.TEST:
      return "litecoin";
  }
}

export function tokenAddressToToken(address: string): TokenE | null {
  switch (address) {
    case "So11111111111111111111111111111111111111112":
      return TokenE.SOL;
    case "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So":
      return TokenE.mSOL;
    case "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj":
      return TokenE.stSOL;
    // case "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU":
    case "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr":
      return TokenE.USDC;
    case "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB":
      return TokenE.USDT;
    case "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R":
      return TokenE.RAY;
    case "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE":
      return TokenE.ORCA;
    case "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263":
      return TokenE.Bonk;
    case "6QGdQbaZEgpXqqbGwXJZXwbZ9xJnthfyYNZ92ARzTdAX":
      return TokenE.TEST;
    default:
      return null;
  }
}

export function getTokenAddress(token: TokenE) {
  switch (token) {
    case TokenE.SOL:
      return "So11111111111111111111111111111111111111112";
    case TokenE.mSOL:
      return "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So";
    case TokenE.stSOL:
      return "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj";
    case TokenE.USDC:
      // return "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
      return "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr";
    case TokenE.USDT:
      return "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
    case TokenE.RAY:
      return "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";
    case TokenE.ORCA:
      return "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE";
    case TokenE.Bonk:
      return "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";
    case TokenE.TEST:
      return "6QGdQbaZEgpXqqbGwXJZXwbZ9xJnthfyYNZ92ARzTdAX";
  }
}
