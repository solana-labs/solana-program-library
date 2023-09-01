"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.TSConfigReader = void 0;
const path_1 = require("path");
const typescript_1 = __importDefault(require("typescript"));
const fs_1 = require("../../fs");
const assert_1 = require("assert");
const validation_1 = require("../../validation");
const paths_1 = require("../../paths");
const module_1 = require("module");
const tsdoc_defaults_1 = require("../tsdoc-defaults");
const array_1 = require("../../array");
const entry_point_1 = require("../../entry-point");
const tsconfig_1 = require("../../tsconfig");
function isSupportForTags(obj) {
    return ((0, validation_1.validate)({}, obj) &&
        Object.entries(obj).every(([key, val]) => {
            return (/^@[a-zA-Z][a-zA-Z0-9]*$/.test(key) && typeof val === "boolean");
        }));
}
const tsDocSchema = {
    $schema: (0, validation_1.optional)(String),
    extends: (0, validation_1.optional)([Array, String]),
    noStandardTags: (0, validation_1.optional)(Boolean),
    tagDefinitions: (0, validation_1.optional)([
        Array,
        {
            tagName: validation_1.isTagString,
            syntaxKind: ["inline", "block", "modifier"],
            allowMultiple: (0, validation_1.optional)(Boolean),
            [validation_1.additionalProperties]: false,
        },
    ]),
    supportForTags: (0, validation_1.optional)(isSupportForTags),
    // The official parser has code to support for these two, but
    // the schema doesn't allow them... just silently ignore them for now.
    supportedHtmlElements: (0, validation_1.optional)({}),
    reportUnsupportedHtmlElements: (0, validation_1.optional)(Boolean),
    [validation_1.additionalProperties]: false,
};
class TSConfigReader {
    constructor() {
        /**
         * Note: Runs after the {@link TypeDocReader}.
         */
        this.order = 200;
        this.name = "tsconfig-json";
        this.supportsPackages = true;
        this.seenTsdocPaths = new Set();
    }
    read(container, logger, cwd) {
        const file = container.getValue("tsconfig") || cwd;
        let fileToRead = (0, tsconfig_1.findTsConfigFile)(file);
        if (!fileToRead) {
            // If the user didn't give us this option, we shouldn't complain about not being able to find it.
            if (container.isSet("tsconfig")) {
                logger.error(`The tsconfig file ${(0, paths_1.nicePath)(file)} does not exist`);
            }
            else if (container.getValue("entryPointStrategy") !==
                entry_point_1.EntryPointStrategy.Packages) {
                logger.warn("No tsconfig file found, this will prevent TypeDoc from finding your entry points.");
            }
            return;
        }
        fileToRead = (0, paths_1.normalizePath)((0, path_1.resolve)(fileToRead));
        logger.verbose(`Reading tsconfig at ${(0, paths_1.nicePath)(fileToRead)}`);
        this.addTagsFromTsdocJson(container, logger, (0, path_1.resolve)(fileToRead));
        const parsed = (0, tsconfig_1.readTsConfig)(fileToRead, logger);
        if (!parsed) {
            return;
        }
        logger.diagnostics(parsed.errors);
        const typedocOptions = parsed.raw?.typedocOptions ?? {};
        if (typedocOptions.options) {
            logger.error([
                "typedocOptions in tsconfig file specifies an option file to read but the option",
                "file has already been read. This is likely a misconfiguration.",
            ].join(" "));
            delete typedocOptions.options;
        }
        if (typedocOptions.tsconfig) {
            logger.error("typedocOptions in tsconfig file may not specify a tsconfig file to read");
            delete typedocOptions.tsconfig;
        }
        container.setCompilerOptions(parsed.fileNames, parsed.options, parsed.projectReferences);
        for (const [key, val] of Object.entries(typedocOptions || {})) {
            try {
                // We catch the error, so can ignore the strict type checks
                container.setValue(key, val, (0, path_1.join)(fileToRead, ".."));
            }
            catch (error) {
                (0, assert_1.ok)(error instanceof Error);
                logger.error(error.message);
            }
        }
    }
    addTagsFromTsdocJson(container, logger, tsconfig) {
        this.seenTsdocPaths.clear();
        const tsdoc = (0, path_1.join)((0, path_1.dirname)(tsconfig), "tsdoc.json");
        if (!(0, fs_1.isFile)(tsdoc)) {
            return;
        }
        const overwritten = ["blockTags", "inlineTags", "modifierTags"].filter((opt) => container.isSet(opt));
        if (overwritten.length) {
            logger.warn(`The ${overwritten.join(", ")} defined in typedoc.json will ` +
                "be overwritten by configuration in tsdoc.json.");
        }
        const config = this.readTsDoc(logger, tsdoc);
        if (!config)
            return;
        const supported = (tag) => {
            return config.supportForTags
                ? !!config.supportForTags[tag.tagName]
                : true;
        };
        const blockTags = [];
        const inlineTags = [];
        const modifierTags = [];
        if (!config.noStandardTags) {
            blockTags.push(...tsdoc_defaults_1.tsdocBlockTags);
            inlineTags.push(...tsdoc_defaults_1.tsdocInlineTags);
            modifierTags.push(...tsdoc_defaults_1.tsdocModifierTags);
        }
        for (const { tagName, syntaxKind } of config.tagDefinitions?.filter(supported) || []) {
            const arr = {
                block: blockTags,
                inline: inlineTags,
                modifier: modifierTags,
            }[syntaxKind];
            arr.push(tagName);
        }
        container.setValue("blockTags", (0, array_1.unique)(blockTags));
        container.setValue("inlineTags", (0, array_1.unique)(inlineTags));
        container.setValue("modifierTags", (0, array_1.unique)(modifierTags));
    }
    readTsDoc(logger, path) {
        if (this.seenTsdocPaths.has(path)) {
            logger.error(`Circular reference encountered for "extends" field of ${(0, paths_1.nicePath)(path)}`);
            return;
        }
        this.seenTsdocPaths.add(path);
        const { config, error } = typescript_1.default.readConfigFile((0, paths_1.normalizePath)(path), typescript_1.default.sys.readFile);
        if (error) {
            logger.error(`Failed to read tsdoc.json file at ${(0, paths_1.nicePath)(path)}.`);
            return;
        }
        if (!(0, validation_1.validate)(tsDocSchema, config)) {
            logger.error(`The file ${(0, paths_1.nicePath)(path)} is not a valid tsdoc.json file.`);
            return;
        }
        const workingConfig = {};
        if (config.extends) {
            const resolver = (0, module_1.createRequire)(path);
            for (const extendedPath of config.extends) {
                let resolvedPath;
                try {
                    resolvedPath = resolver.resolve(extendedPath);
                }
                catch {
                    logger.error(`Failed to resolve ${extendedPath} to a file in ${(0, paths_1.nicePath)(path)}`);
                    return;
                }
                const parentConfig = this.readTsDoc(logger, resolvedPath);
                if (!parentConfig)
                    return;
                mergeConfigs(parentConfig, workingConfig);
            }
        }
        mergeConfigs(config, workingConfig);
        return workingConfig;
    }
}
exports.TSConfigReader = TSConfigReader;
function mergeConfigs(from, into) {
    if (from.supportForTags) {
        into.supportForTags || (into.supportForTags = {});
        Object.assign(into.supportForTags, from.supportForTags);
    }
    if (from.tagDefinitions) {
        into.tagDefinitions || (into.tagDefinitions = []);
        into.tagDefinitions.push(...from.tagDefinitions);
    }
    into.noStandardTags = from.noStandardTags ?? into.noStandardTags;
}
