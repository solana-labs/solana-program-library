
const SAMPLEWALLET = "D6R2h5zXaprFyKNh2QwqiY4ZvW6TE5cfyrCPTWvQdLcc";
export const walletShortener = (addr: string = SAMPLEWALLET) =>
  addr.substring(0, 5) + "..." + addr.substring(addr.length - 4);

export const BannerImageUrl =  "https://s3-alpha-sig.figma.com/img/883a/39c1/b56392e632dacf92f1bed1a44f046ff5?Expires=1687737600&Signature=QDjfHKQ-AqLm2u9Ah90YQRnrfgXYuDV568GBXuDtJa-AxBkXJzBzjaNSR1bYz98zP0fynOWHJIGP4nqaiaLHOZbgqY~B90d5nAX9ACax3xDCCjS6Bnk1cUUE5jVcJag3RizrjcWJjG1jNWI0jRpLYMBQFlwfu1R~Kj4iFhvMnHXT1XXFEVKcYeLwEZ7nsZiEC7tg99NaN6l52WdA3ETbHVgsoc208unAhBaLSbxc-XBcuF51AwUzFh4NLuHT61giqULIkfrFMf2qYBoEKLCTGBJwapuOzTesCpZlcbcAkoVuY7p-zPgWtMbjUJ8F9~7Bn1SVfVgq6Ng3ASCqseNSuw__&Key-Pair-Id=APKAQ4GOSFWCVNEHN3O4"