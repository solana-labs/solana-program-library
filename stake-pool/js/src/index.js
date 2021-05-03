var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import * as schema from './schema.js';
import borsh from "borsh";
import solanaWeb3 from '@solana/web3.js';
const STAKE_POOL_ACCT_LENGTH = 298;
const connection = new solanaWeb3.Connection("https://devnet.solana.com/", 'confirmed');
const STAKE_POOL_ADDR = new solanaWeb3.PublicKey("poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj");
getStakePoolAccounts(connection, STAKE_POOL_ADDR).then((accounts) => {
    accounts.map((sp) => {
        if (sp) {
            for (const val in sp) {
                if (sp[val] instanceof schema.PublicKey) {
                    console.log(val, new solanaWeb3.PublicKey(new solanaWeb3.PublicKey(sp[val].value).toBytes().reverse()).toString());
                }
                else {
                    console.log(val, sp[val]);
                }
            }
        }
        console.log('\n');
    });
});
function decodeSerializedStakePool(serializedStakePool, accountType) {
    const stakePoolSchema = schema.constructStakePoolSchema();
    const stakePool = new schema.StakePool(borsh.deserializeUnchecked(stakePoolSchema, accountType, serializedStakePool));
    return stakePool;
}
function getStakePoolAccounts(connection, stakePoolAddress) {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            let response = yield connection.getProgramAccounts(STAKE_POOL_ADDR);
            /*
            if (!response.ok) {
                throw new Error(`Can't get list of Stake Pool accounts associated with ${STAKE_POOL_ADDR}`)
            }
            */
            const stakePoolAccounts = response.map((a) => {
                if (a.account.data.length === STAKE_POOL_ACCT_LENGTH) {
                    return decodeSerializedStakePool(a.account.data, schema.StakePool);
                }
                else {
                    return decodeSerializedStakePool(a.account.data, schema.ValidatorList);
                }
            });
            return stakePoolAccounts;
        }
        catch (error) {
            console.log(error);
        }
    });
}
