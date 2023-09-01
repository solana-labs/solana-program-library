"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.crypto = void 0;
const nc = require("node:crypto");
exports.crypto = nc && typeof nc === 'object' && 'webcrypto' in nc ? nc.webcrypto : undefined;
//# sourceMappingURL=cryptoNode.js.map