"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getSigners = void 0;
const web3_js_1 = require("@solana/web3.js");
/** @internal */
function getSigners(signerOrMultisig, multiSigners) {
    return signerOrMultisig instanceof web3_js_1.PublicKey
        ? [signerOrMultisig, multiSigners]
        : [signerOrMultisig.publicKey, [signerOrMultisig]];
}
exports.getSigners = getSigners;
//# sourceMappingURL=internal.js.map