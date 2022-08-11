const orca_sdk = require("@orca-so/sdk");
const solana = require("@solana/web3.js");
const fs = require("fs");

function replacer(key, value) {
  if (typeof value === "object" && "_bn" in value) {
    return value.toString();
  }
  return value;
}

const getPools = () => {
  const connection = new solana.Connection(
    "https://api.mainnet-beta.solana.com",
    "singleGossip"
  );
  const orca = orca_sdk.getOrca(connection);

  let pools = new Array();
  for (const [key, value] of Object.entries(orca_sdk.OrcaPoolConfig)) {
    const orcaPool = orca.getPool(value);
    pools.push({ ...{ name: key }, ...orcaPool.poolParams });
  }
  console.log(
    '{\n"name": "Orca Pools",\n"pools":\n',
    JSON.stringify(pools, replacer, 2),
    "\n}"
  );
};

const getFarms = () => {
  const connection = new solana.Connection(
    "https://api.mainnet-beta.solana.com",
    "singleGossip"
  );
  const orca = orca_sdk.getOrca(connection);

  let farms = new Array();
  for (const [key, value] of Object.entries(orca_sdk.OrcaFarmConfig)) {
    const orcaFarm = orca.getFarm(value);
    farms.push({ ...{ name: key }, ...orcaFarm.farmParams });
  }
  console.log(
    '{\n"name": "Orca Farms",\n"farms":\n',
    JSON.stringify(farms, replacer, 2),
    "\n}"
  );
};

if (process.argv.length > 2) {
  if (process.argv[2] == "get_pools") {
    getPools();
  } else if (process.argv[2] == "get_farms") {
    getFarms();
  } else {
    console.error("Use it with get_pools or get_farms argument");
  }
} else {
  console.error("Use it with get_pools or get_farms argument");
}
