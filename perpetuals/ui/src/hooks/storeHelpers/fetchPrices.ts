import { TOKEN_LIST, getTokenId } from "@/lib/Token";
import { PriceStats } from "@/lib/types";

type FetchedData = {
  [key: string]: {
    usd: number;
    usd_24h_vol: number;
    usd_24h_change: number;
  };
};

export function fetchAllStats(): PriceStats {
  let stats = fetch(
    `https://api.coingecko.com/api/v3/simple/price?ids=${TOKEN_LIST.map(
      getTokenId
    ).join(
      ","
    )}&vs_currencies=USD&include_24hr_vol=true&include_24hr_change=true`
  )
    .then((resp) => resp.json())
    .then((data: FetchedData) => {
      const allStats = TOKEN_LIST.reduce((acc, token) => {
        const tokenData = data[getTokenId(token)];

        acc[token] = {
          change24hr: tokenData!.usd_24h_change,
          currentPrice: tokenData!.usd,
          high24hr: 0,
          low24hr: 0,
        };

        return acc;
      }, {} as PriceStats);

      return allStats;
    })
    .catch(() => {
      console.log("caught data fetching error");
      const allStats = TOKEN_LIST.reduce((acc, token) => {
        acc[token] = {
          change24hr: 0,
          currentPrice: 0,
          high24hr: 0,
          low24hr: 0,
        };

        return acc;
      }, {} as PriceStats);

      return allStats;
    });

  return stats;
}
