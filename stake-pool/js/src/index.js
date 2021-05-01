import * as schema from './schema.js'
import borsh from "borsh"
import solanaWeb3 from '@solana/web3.js';

const connection = new solanaWeb3.Connection("https://testnet.solana.com/", 'confirmed');
// console.log(connection);

const STAKE_POOL_ADDR = new solanaWeb3.PublicKey("poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj")

connection.getProgramAccounts(STAKE_POOL_ADDR).then((e) => {
    // console.log(e)
    // TODO: an account here can either be Unintialized, StakePool, or ValidatorList, 
    // https://github.com/solana-labs/solana-program-library/blob/master/stake-pool/program/src/state.rs#L13
    // TODO Deserialise the state of the StakePool accounts so I can get total_stake_lamports and validatorlist

    // const accounts_data = e.map((account) => console.log(decodeSerializedStakePool(account.account.data)))

    console.log(e[0].account.data)
    const accounts_data = decodeSerializedStakePool(e[0].account.data)
    console.log(accounts_data)
})

function decodeSerializedStakePool(serializedStakePool) {
    const stakePoolSchema = schema.constructStakePoolSchema()
    console.log(stakePoolSchema)
    return borsh.deserializeUnchecked(stakePoolSchema, schema.StakePool, serializedStakePool)
}