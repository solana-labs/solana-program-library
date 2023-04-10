import { TokenE } from "@/lib/Token";

export class UserAccount {
  public lpBalances: Record<string, number>;
  public tokenBalances: Record<TokenE, number>;

  constructor(
    lpBalances: Record<string, number> = {},
    tokenBalances: Record<string, number> = {}
  ) {
    this.lpBalances = lpBalances;
    this.tokenBalances = tokenBalances;
  }

  getUserLpBalance(poolAddress: string): number {
    return this.lpBalances[poolAddress] || 0;
  }
}
