"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.resolveAliasedSymbol = void 0;
const typescript_1 = __importDefault(require("typescript"));
function resolveAliasedSymbol(symbol, checker) {
    while (typescript_1.default.SymbolFlags.Alias & symbol.flags) {
        symbol = checker.getAliasedSymbol(symbol);
    }
    return symbol;
}
exports.resolveAliasedSymbol = resolveAliasedSymbol;
