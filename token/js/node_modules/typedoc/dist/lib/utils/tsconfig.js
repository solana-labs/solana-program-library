"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.readTsConfig = exports.findTsConfigFile = void 0;
const typescript_1 = __importDefault(require("typescript"));
const fs_1 = require("./fs");
function findTsConfigFile(path) {
    let fileToRead = path;
    if ((0, fs_1.isDir)(fileToRead)) {
        fileToRead = typescript_1.default.findConfigFile(path, fs_1.isFile);
    }
    if (!fileToRead || !(0, fs_1.isFile)(fileToRead)) {
        return;
    }
    return fileToRead;
}
exports.findTsConfigFile = findTsConfigFile;
const tsConfigCache = {};
function readTsConfig(path, logger) {
    if (tsConfigCache[path]) {
        return tsConfigCache[path];
    }
    const parsed = typescript_1.default.getParsedCommandLineOfConfigFile(path, {}, {
        ...typescript_1.default.sys,
        onUnRecoverableConfigFileDiagnostic: logger.diagnostic.bind(logger),
    });
    if (!parsed) {
        return;
    }
    logger.diagnostics(parsed.errors);
    tsConfigCache[path] = parsed;
    return parsed;
}
exports.readTsConfig = readTsConfig;
