"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.createAssociatedTokenAccount = void 0;
const web3_js_1 = require("@solana/web3.js");
const constants_js_1 = require("../constants.js");
const associatedTokenAccount_js_1 = require("../instructions/associatedTokenAccount.js");
const mint_js_1 = require("../state/mint.js");
/**
 * Create and initialize a new associated token account
 *
 * @param connection               Connection to use
 * @param payer                    Payer of the transaction and initialization fees
 * @param mint                     Mint for the account
 * @param owner                    Owner of the new account
 * @param confirmOptions           Options for confirming the transaction
 * @param programId                SPL Token program account
 * @param associatedTokenProgramId SPL Associated Token program account
 *
 * @return Address of the new associated token account
 */
function createAssociatedTokenAccount(connection, payer, mint, owner, confirmOptions, programId = constants_js_1.TOKEN_PROGRAM_ID, associatedTokenProgramId = constants_js_1.ASSOCIATED_TOKEN_PROGRAM_ID) {
    return __awaiter(this, void 0, void 0, function* () {
        const associatedToken = (0, mint_js_1.getAssociatedTokenAddressSync)(mint, owner, false, programId, associatedTokenProgramId);
        const transaction = new web3_js_1.Transaction().add((0, associatedTokenAccount_js_1.createAssociatedTokenAccountInstruction)(payer.publicKey, associatedToken, owner, mint, programId, associatedTokenProgramId));
        yield (0, web3_js_1.sendAndConfirmTransaction)(connection, transaction, [payer], confirmOptions);
        return associatedToken;
    });
}
exports.createAssociatedTokenAccount = createAssociatedTokenAccount;
//# sourceMappingURL=createAssociatedTokenAccount.js.map