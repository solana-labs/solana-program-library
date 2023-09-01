"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.addSigners = void 0;
const web3_js_1 = require("@solana/web3.js");
/** @internal */
function addSigners(keys, ownerOrAuthority, multiSigners) {
    if (multiSigners.length) {
        keys.push({ pubkey: ownerOrAuthority, isSigner: false, isWritable: false });
        for (const signer of multiSigners) {
            keys.push({
                pubkey: signer instanceof web3_js_1.PublicKey ? signer : signer.publicKey,
                isSigner: true,
                isWritable: false,
            });
        }
    }
    else {
        keys.push({ pubkey: ownerOrAuthority, isSigner: true, isWritable: false });
    }
    return keys;
}
exports.addSigners = addSigners;
//# sourceMappingURL=internal.js.map