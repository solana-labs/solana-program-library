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
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.TypeDocReader = void 0;
const path_1 = require("path");
const FS = __importStar(require("fs"));
const typescript_1 = __importDefault(require("typescript"));
const assert_1 = require("assert");
const paths_1 = require("../../paths");
const fs_1 = require("../../fs");
const module_1 = require("module");
/**
 * Obtains option values from typedoc.json
 * or typedoc.js (discouraged since ~0.14, don't fully deprecate until API has stabilized)
 */
class TypeDocReader {
    constructor() {
        /**
         * Should run before the tsconfig reader so that it can specify a tsconfig file to read.
         */
        this.order = 100;
        this.name = "typedoc-json";
        this.supportsPackages = true;
    }
    /**
     * Read user configuration from a typedoc.json or typedoc.js configuration file.
     */
    read(container, logger, cwd) {
        const path = container.getValue("options") || cwd;
        const file = this.findTypedocFile(path);
        if (!file) {
            if (container.isSet("options")) {
                logger.error(`The options file ${(0, paths_1.nicePath)(path)} does not exist.`);
            }
            return;
        }
        const seen = new Set();
        this.readFile(file, container, logger, seen);
    }
    /**
     * Read the given options file + any extended files.
     * @param file
     * @param container
     * @param logger
     */
    readFile(file, container, logger, seen) {
        if (seen.has(file)) {
            logger.error(`Tried to load the options file ${(0, paths_1.nicePath)(file)} multiple times.`);
            return;
        }
        seen.add(file);
        let fileContent;
        if (file.endsWith(".json")) {
            const readResult = typescript_1.default.readConfigFile((0, paths_1.normalizePath)(file), (path) => FS.readFileSync(path, "utf-8"));
            if (readResult.error) {
                logger.error(`Failed to parse ${(0, paths_1.nicePath)(file)}, ensure it exists and contains an object.`);
                return;
            }
            else {
                fileContent = readResult.config;
            }
        }
        else {
            // eslint-disable-next-line @typescript-eslint/no-var-requires
            fileContent = require(file);
        }
        if (typeof fileContent !== "object" || !fileContent) {
            logger.error(`The root value of ${(0, paths_1.nicePath)(file)} is not an object.`);
            return;
        }
        // clone option object to avoid of property changes in re-calling this file
        const data = { ...fileContent };
        delete data["$schema"]; // Useful for better autocompletion, should not be read as a key.
        if ("extends" in data) {
            const resolver = (0, module_1.createRequire)(file);
            const extended = getStringArray(data["extends"]);
            for (const extendedFile of extended) {
                let resolvedParent;
                try {
                    resolvedParent = resolver.resolve(extendedFile);
                }
                catch {
                    logger.error(`Failed to resolve ${extendedFile} to a file in ${(0, paths_1.nicePath)(file)}`);
                    continue;
                }
                this.readFile(resolvedParent, container, logger, seen);
            }
            delete data["extends"];
        }
        for (const [key, val] of Object.entries(data)) {
            try {
                container.setValue(key, val, (0, path_1.resolve)((0, path_1.dirname)(file)));
            }
            catch (error) {
                (0, assert_1.ok)(error instanceof Error);
                logger.error(error.message);
            }
        }
    }
    /**
     * Search for the configuration file given path
     *
     * @param  path Path to the typedoc.(js|json) file. If path is a directory
     *   typedoc file will be attempted to be found at the root of this path
     * @param logger
     * @return the typedoc.(js|json) file path or undefined
     */
    findTypedocFile(path) {
        path = (0, path_1.resolve)(path);
        return [
            path,
            (0, path_1.join)(path, "typedoc.json"),
            (0, path_1.join)(path, "typedoc.jsonc"),
            (0, path_1.join)(path, "typedoc.config.js"),
            (0, path_1.join)(path, "typedoc.config.cjs"),
            (0, path_1.join)(path, "typedoc.js"),
            (0, path_1.join)(path, "typedoc.cjs"),
            (0, path_1.join)(path, ".config/typedoc.json"),
            (0, path_1.join)(path, ".config/typedoc.jsonc"),
            (0, path_1.join)(path, ".config/typedoc.config.js"),
            (0, path_1.join)(path, ".config/typedoc.config.cjs"),
            (0, path_1.join)(path, ".config/typedoc.js"),
            (0, path_1.join)(path, ".config/typedoc.cjs"),
        ].find(fs_1.isFile);
    }
}
exports.TypeDocReader = TypeDocReader;
function getStringArray(arg) {
    return Array.isArray(arg) ? arg.map(String) : [String(arg)];
}
