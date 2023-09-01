"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.isObjectType = exports.getHeritageTypes = exports.isNamedNode = void 0;
const typescript_1 = __importDefault(require("typescript"));
function isNamedNode(node) {
    const name = node.name;
    return !!name && (typescript_1.default.isMemberName(name) || typescript_1.default.isComputedPropertyName(name));
}
exports.isNamedNode = isNamedNode;
function getHeritageTypes(declarations, kind) {
    const exprs = declarations.flatMap((d) => (d.heritageClauses ?? [])
        .filter((hc) => hc.token === kind)
        .flatMap((hc) => hc.types));
    const seenTexts = new Set();
    return exprs.filter((expr) => {
        const text = expr.getText();
        if (seenTexts.has(text)) {
            return false;
        }
        seenTexts.add(text);
        return true;
    });
}
exports.getHeritageTypes = getHeritageTypes;
function isObjectType(type) {
    return typeof type.objectFlags === "number";
}
exports.isObjectType = isObjectType;
