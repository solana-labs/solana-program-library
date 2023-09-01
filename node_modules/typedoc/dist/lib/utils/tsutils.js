"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.getHumanName = exports.getQualifiedName = void 0;
const ts = __importStar(require("typescript"));
function getQualifiedName(symbol, defaultName) {
    // Two implementation options for this one:
    // 1. Use the internal symbol.parent, to walk up until we hit a source file symbol (if in a module)
    //    or undefined (if in a global file)
    // 2. Use checker.getFullyQualifiedName and parse out the name from the returned string.
    // The symbol.parent method is easier to check for now.
    let sym = symbol;
    const parts = [];
    while (sym && !sym.declarations?.some(ts.isSourceFile)) {
        parts.unshift(getHumanName(sym.name));
        sym = sym.parent;
    }
    return parts.join(".") || defaultName;
}
exports.getQualifiedName = getQualifiedName;
function getHumanName(name) {
    // Unique symbols get a name that will change between runs of the compiler.
    const match = /^__@(.*)@\d+$/.exec(name);
    if (match) {
        return `[${match[1]}]`;
    }
    return name;
}
exports.getHumanName = getHumanName;
