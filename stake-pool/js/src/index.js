import constructStakePoolSchema from './schema.js'
import * as borsh from "borsh"

console.log("Hello!");
console.log(solanaWeb3);
console.log("Bye!");

function decodeSerializedStakePool(serializedStakePool) {
    const stakePoolSchema = constructStakePoolSchema()
    return borsh.deserialize(stakePoolSchema, 'StakePool', serializedStakePool)
}

const STAKE_POOL_ADDR = new solanaWeb3.PublicKey("poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj")
console.log(STAKE_POOL_ADDR)

const connection = new solanaWeb3.Connection("https://testnet.solana.com/");
console.log(connection);

connection.getProgramAccounts(STAKE_POOL_ADDR).then((e) => {
    console.log(e)
    // TODO: an account here can either be Unintialized, StakePool, or ValidatorList, 
    // https://github.com/solana-labs/solana-program-library/blob/master/stake-pool/program/src/state.rs#L13
    // TODO Deserialise the state of the StakePool accounts so I can get total_stake_lamports and validatorlist

    e.map((account) => { console.log(decodeSerializedStakePool(account.account.data)) })
})