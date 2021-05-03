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
import solanaWeb3 from '@solana/web3.js';
export class StakePool {
}
function decodeSerializedStakePool(serializedStakePool, accountType) {
    return accountType.decode(serializedStakePool);
}
function getStakePoolAccounts(connection, stakePoolAddress) {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            let response = yield connection.getProgramAccounts(STAKE_POOL_ADDR);
            const stakePoolAccounts = response.map(a => {
                let b = {
                    pubkey: a.pubkey,
                    account: {
                        data: null,
                        executable: a.account.executable,
                        lamports: a.account.lamports,
                        owner: a.account.owner,
                    },
                };
                if (a.account.data.length === STAKE_POOL_ACCT_LENGTH) {
                    b.account.data = decodeSerializedStakePool(a.account.data, schema.StakePoolAccount);
                }
                else {
                    b.account.data = decodeSerializedStakePool(a.account.data, schema.ValidatorListAccount);
                }
                return b;
            });
            return stakePoolAccounts;
        }
        catch (error) {
            console.log(error);
        }
    });
}
/* Test function on devnet: get accounts, deserialize them, then log them */
const STAKE_POOL_ACCT_LENGTH = 298;
const connection = new solanaWeb3.Connection('https://devnet.solana.com/', 'confirmed');
const STAKE_POOL_ADDR = new solanaWeb3.PublicKey('poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj');
getStakePoolAccounts(connection, STAKE_POOL_ADDR).then(accounts => {
    accounts.map(sp => {
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
