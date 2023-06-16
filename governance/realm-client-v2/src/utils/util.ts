
const SAMPLEWALLET = "D6R2h5zXaprFyKNh2QwqiY4ZvW6TE5cfyrCPTWvQdLcc";
export const walletShortener = (addr: string = SAMPLEWALLET) =>
  addr.substring(0, 5) + "..." + addr.substring(addr.length - 4);