"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.removeUndefined = void 0;
const models_1 = require("../../models");
function removeUndefined(type) {
    if (type instanceof models_1.UnionType) {
        const types = type.types.filter((t) => {
            if (t instanceof models_1.IntrinsicType) {
                return t.name !== "undefined";
            }
            return true;
        });
        if (types.length === 1) {
            return types[0];
        }
        type.types = types;
        return type;
    }
    return type;
}
exports.removeUndefined = removeUndefined;
