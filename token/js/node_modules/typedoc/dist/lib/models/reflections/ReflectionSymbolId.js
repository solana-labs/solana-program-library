"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.ReflectionSymbolId = void 0;
const fs_1 = require("fs");
const path_1 = require("path");
const typescript_1 = __importDefault(require("typescript"));
const fs_2 = require("../../utils/fs");
const tsutils_1 = require("../../utils/tsutils");
const validation_1 = require("../../utils/validation");
const paths_1 = require("../../utils/paths");
/**
 * This exists so that TypeDoc can store a unique identifier for a `ts.Symbol` without
 * keeping a reference to the `ts.Symbol` itself. This identifier should be stable across
 * runs so long as the symbol is exported from the same file.
 */
class ReflectionSymbolId {
    constructor(symbol, declaration) {
        if ("name" in symbol) {
            declaration ?? (declaration = symbol?.declarations?.[0]);
            this.fileName = (0, paths_1.normalizePath)(declaration?.getSourceFile().fileName ?? "\0");
            if (symbol.declarations?.some(typescript_1.default.isSourceFile)) {
                this.qualifiedName = "";
            }
            else {
                this.qualifiedName = (0, tsutils_1.getQualifiedName)(symbol, symbol.name);
            }
            this.pos = declaration?.pos ?? Infinity;
        }
        else {
            this.fileName = symbol.sourceFileName;
            this.qualifiedName = symbol.qualifiedName;
            this.pos = Infinity;
        }
    }
    getStableKey() {
        if (Number.isFinite(this.pos)) {
            return `${this.fileName}\0${this.qualifiedName}\0${this.pos}`;
        }
        else {
            return `${this.fileName}\0${this.qualifiedName}`;
        }
    }
    toObject(serializer) {
        return {
            sourceFileName: (0, path_1.isAbsolute)(this.fileName)
                ? (0, paths_1.normalizePath)((0, path_1.relative)(serializer.projectRoot, resolveDeclarationMaps(this.fileName)))
                : this.fileName,
            qualifiedName: this.qualifiedName,
        };
    }
}
exports.ReflectionSymbolId = ReflectionSymbolId;
const declarationMapCache = new Map();
/**
 * See also getTsSourceFromJsSource in package-manifest.ts.
 */
function resolveDeclarationMaps(file) {
    if (!file.endsWith(".d.ts"))
        return file;
    if (declarationMapCache.has(file))
        return declarationMapCache.get(file);
    const mapFile = file + ".map";
    if (!(0, fs_1.existsSync)(mapFile))
        return file;
    let sourceMap;
    try {
        sourceMap = JSON.parse((0, fs_2.readFile)(mapFile));
    }
    catch {
        return file;
    }
    if ((0, validation_1.validate)({
        file: String,
        sourceRoot: (0, validation_1.optional)(String),
        sources: [Array, String],
    }, sourceMap)) {
        // There's a pretty large assumption in here that we only have
        // 1 source file per js file. This is a pretty standard typescript approach,
        // but people might do interesting things with transpilation that could break this.
        let source = sourceMap.sources[0];
        // If we have a sourceRoot, trim any leading slash from the source, and join them
        // Similar to how it's done at https://github.com/mozilla/source-map/blob/58819f09018d56ef84dc41ba9c93f554e0645169/lib/util.js#L412
        if (sourceMap.sourceRoot !== undefined) {
            source = source.replace(/^\//, "");
            source = (0, path_1.join)(sourceMap.sourceRoot, source);
        }
        const result = (0, path_1.resolve)(mapFile, "..", source);
        declarationMapCache.set(file, result);
        return result;
    }
    return file;
}
