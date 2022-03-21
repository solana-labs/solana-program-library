// @flow
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
import { Keypair } from '@solana/web3.js';
import { sleep } from './sleep';
export function newAccountWithLamports(connection, lamports = 1000000) {
    return __awaiter(this, void 0, void 0, function* () {
        const account = Keypair.generate();
        let retries = 30;
        yield connection.requestAirdrop(account.publicKey, lamports);
        for (;;) {
            yield sleep(500);
            if (lamports == (yield connection.getBalance(account.publicKey))) {
                return account;
            }
            if (--retries <= 0) {
                break;
            }
        }
        throw new Error(`Airdrop of ${lamports} failed`);
    });
}
