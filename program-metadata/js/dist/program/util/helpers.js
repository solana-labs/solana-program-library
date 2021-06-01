"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.startCase = void 0;
const startCase = (str) => {
    const result = str.replace(/([A-Z])/g, " $1");
    return result.charAt(0).toUpperCase() + result.slice(1);
};
exports.startCase = startCase;
//# sourceMappingURL=helpers.js.map