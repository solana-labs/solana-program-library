"use strict";
// Utilities to support the inspection of node package "manifests"
Object.defineProperty(exports, "__esModule", { value: true });
exports.getTsEntryPointForPackage = exports.ignorePackage = exports.expandPackages = exports.extractTypedocConfigFromPackageManifest = exports.loadPackageManifest = void 0;
const path_1 = require("path");
const fs_1 = require("fs");
const fs_2 = require("./fs");
const paths_1 = require("./paths");
const validation_1 = require("./validation");
/**
 * Helper for the TS type system to understand hasOwnProperty
 * and narrow a type appropriately.
 * @param obj the receiver of the hasOwnProperty method call
 * @param prop the property to test for
 */
function hasOwnProperty(obj, prop) {
    return Object.prototype.hasOwnProperty.call(obj, prop);
}
/**
 * Loads a package.json and validates that it is a JSON Object
 */
function loadPackageManifest(logger, packageJsonPath) {
    const packageJson = JSON.parse((0, fs_2.readFile)(packageJsonPath));
    if (typeof packageJson !== "object" || !packageJson) {
        logger.error(`The file ${packageJsonPath} is not an object.`);
        return undefined;
    }
    return packageJson;
}
exports.loadPackageManifest = loadPackageManifest;
const typedocPackageManifestConfigSchema = {
    displayName: (0, validation_1.optional)(String),
    entryPoint: (0, validation_1.optional)(String),
    readmeFile: (0, validation_1.optional)(String),
    tsconfig: (0, validation_1.optional)(String),
    [validation_1.additionalProperties]: false,
};
/**
 * Extracts typedoc specific config from a specified package manifest
 */
function extractTypedocConfigFromPackageManifest(logger, packageJsonPath) {
    const packageJson = loadPackageManifest(logger, packageJsonPath);
    if (!packageJson) {
        return undefined;
    }
    if (hasOwnProperty(packageJson, "typedoc") &&
        typeof packageJson["typedoc"] == "object" &&
        packageJson["typedoc"]) {
        if (!(0, validation_1.validate)(typedocPackageManifestConfigSchema, packageJson["typedoc"])) {
            logger.error(`Typedoc config extracted from package manifest file ${packageJsonPath} is not valid`);
            return undefined;
        }
        return packageJson["typedoc"];
    }
    return undefined;
}
exports.extractTypedocConfigFromPackageManifest = extractTypedocConfigFromPackageManifest;
/**
 * Load the paths to packages specified in a Yarn workspace package JSON
 * Returns undefined if packageJSON does not define a Yarn workspace
 * @param packageJSON the package json object
 */
function getPackagePaths(packageJSON) {
    if (Array.isArray(packageJSON["workspaces"]) &&
        packageJSON["workspaces"].every((i) => typeof i === "string")) {
        return packageJSON["workspaces"];
    }
    if (typeof packageJSON["workspaces"] === "object" &&
        packageJSON["workspaces"] != null) {
        const workspaces = packageJSON["workspaces"];
        if (hasOwnProperty(workspaces, "packages") &&
            Array.isArray(workspaces["packages"]) &&
            workspaces["packages"].every((i) => typeof i === "string")) {
            return workspaces["packages"];
        }
    }
    return undefined;
}
/**
 * Given a list of (potentially wildcarded) package paths,
 * return all the actual package folders found.
 */
function expandPackages(logger, packageJsonDir, workspaces, exclude) {
    // Technically npm and Yarn workspaces don't support recursive nesting,
    // however we support the passing of paths to either packages or
    // to the root of a workspace tree in our params and so we could here
    // be dealing with either a root or a leaf. So let's do this recursively,
    // as it actually is simpler from an implementation perspective anyway.
    return workspaces.flatMap((workspace) => {
        const globbedPackageJsonPaths = (0, fs_2.glob)((0, path_1.resolve)(packageJsonDir, workspace, "package.json"), (0, path_1.resolve)(packageJsonDir));
        if (globbedPackageJsonPaths.length === 0) {
            logger.warn(`The entrypoint glob ${(0, paths_1.nicePath)(workspace)} did not match any directories containing package.json.`);
        }
        else {
            logger.verbose(`Expanded ${(0, paths_1.nicePath)(workspace)} to:\n\t${globbedPackageJsonPaths.map(paths_1.nicePath).join("\n\t")}`);
        }
        return globbedPackageJsonPaths.flatMap((packageJsonPath) => {
            if ((0, paths_1.matchesAny)(exclude, (0, path_1.dirname)(packageJsonPath))) {
                return [];
            }
            const packageJson = loadPackageManifest(logger, packageJsonPath);
            if (packageJson === undefined) {
                logger.error(`Failed to load ${packageJsonPath}`);
                return [];
            }
            const packagePaths = getPackagePaths(packageJson);
            if (packagePaths === undefined) {
                // Assume this is a single package repo
                return [(0, path_1.dirname)(packageJsonPath)];
            }
            // This is a workspace root package, recurse
            return expandPackages(logger, (0, path_1.dirname)(packageJsonPath), packagePaths, exclude);
        });
    });
}
exports.expandPackages = expandPackages;
/**
 * Finds the corresponding TS file from a transpiled JS file.
 * The JS must be built with sourcemaps.
 */
function getTsSourceFromJsSource(logger, jsPath) {
    const contents = (0, fs_2.readFile)(jsPath);
    const sourceMapPrefix = "\n//# sourceMappingURL=";
    const indexOfSourceMapPrefix = contents.indexOf(sourceMapPrefix);
    if (indexOfSourceMapPrefix === -1) {
        logger.verbose(`The file ${jsPath} does not contain a sourceMappingURL`);
        return jsPath;
    }
    const endOfSourceMapPrefix = indexOfSourceMapPrefix + sourceMapPrefix.length;
    const newLineIndex = contents.indexOf("\n", endOfSourceMapPrefix);
    const sourceMapURL = contents.slice(endOfSourceMapPrefix, newLineIndex === -1 ? undefined : newLineIndex);
    let resolvedSourceMapURL;
    let sourceMap;
    if (sourceMapURL.startsWith("data:application/json;base64,")) {
        resolvedSourceMapURL = jsPath;
        sourceMap = JSON.parse(Buffer.from(sourceMapURL.substring(sourceMapURL.indexOf(",") + 1), "base64").toString());
    }
    else {
        resolvedSourceMapURL = (0, path_1.resolve)(jsPath, "..", sourceMapURL);
        sourceMap = JSON.parse((0, fs_2.readFile)(resolvedSourceMapURL));
    }
    if (typeof sourceMap !== "object" || !sourceMap) {
        logger.error(`The source map file ${resolvedSourceMapURL} is not an object.`);
        return undefined;
    }
    if (!hasOwnProperty(sourceMap, "sources") ||
        !Array.isArray(sourceMap.sources)) {
        logger.error(`The source map ${resolvedSourceMapURL} does not contain "sources".`);
        return undefined;
    }
    let sourceRoot;
    if (hasOwnProperty(sourceMap, "sourceRoot") &&
        typeof sourceMap.sourceRoot === "string") {
        sourceRoot = sourceMap.sourceRoot;
    }
    // There's a pretty large assumption in here that we only have
    // 1 source file per js file. This is a pretty standard typescript approach,
    // but people might do interesting things with transpilation that could break this.
    let source = sourceMap.sources[0];
    // If we have a sourceRoot, trim any leading slash from the source, and join them
    // Similar to how it's done at https://github.com/mozilla/source-map/blob/58819f09018d56ef84dc41ba9c93f554e0645169/lib/util.js#L412
    if (sourceRoot !== undefined) {
        source = source.replace(/^\//, "");
        source = (0, path_1.join)(sourceRoot, source);
    }
    const sourcePath = (0, path_1.resolve)(resolvedSourceMapURL, "..", source);
    return sourcePath;
}
// A Symbol used to communicate that this package should be ignored
exports.ignorePackage = Symbol("ignorePackage");
/**
 * Given a package.json, attempt to find the TS file that defines its entry point
 * The JS must be built with sourcemaps.
 *
 * When the TS file cannot be determined, the intention is to
 * - Ignore things which don't appear to be `require`-able node packages.
 * - Fail on things which appear to be `require`-able node packages but are missing
 *   the necessary metadata for us to document.
 */
function getTsEntryPointForPackage(logger, packageJsonPath, packageJson) {
    let packageMain = "index.js"; // The default, per the npm docs.
    let packageTypes = null;
    const typedocPackageConfig = extractTypedocConfigFromPackageManifest(logger, packageJsonPath);
    if (typedocPackageConfig?.entryPoint) {
        packageMain = typedocPackageConfig.entryPoint;
    }
    else if ((0, validation_1.validate)({ typedocMain: String }, packageJson)) {
        logger.warn(`Legacy typedoc entry point config (using "typedocMain" field) found for "${(0, paths_1.nicePath)(packageJsonPath)}". Please update to use "typedoc": { "entryPoint": "..." } instead. In future upgrade, "typedocMain" field will be ignored.`);
        packageMain = packageJson.typedocMain;
    }
    else if ((0, validation_1.validate)({ main: String }, packageJson)) {
        packageMain = packageJson.main;
    }
    else if ((0, validation_1.validate)({ types: String }, packageJson)) {
        packageTypes = packageJson.types;
    }
    else if ((0, validation_1.validate)({ typings: String }, packageJson)) {
        packageTypes = packageJson.typings;
    }
    let entryPointPath = (0, path_1.resolve)(packageJsonPath, "..", packageMain);
    // The entryPointPath from the package manifest can be like a require path.
    // It could end with .js, or it could end without .js, or it could be a folder containing an index.js
    // We can use require.resolve to let node do its magic.
    // Pass an empty `paths` as node_modules locations do not need to be examined
    try {
        entryPointPath = require.resolve(entryPointPath, { paths: [] });
        if ((0, fs_2.hasTsExtension)(entryPointPath) && (0, fs_1.existsSync)(entryPointPath)) {
            return entryPointPath;
        }
    }
    catch (e) {
        if (e.code !== "MODULE_NOT_FOUND") {
            throw e;
        }
        else {
            entryPointPath = (0, path_1.resolve)(packageJsonPath, "..", packageTypes ?? packageMain);
            if (/\.([cm]?[tj]s|tsx?)$/.test(entryPointPath) &&
                (0, fs_1.existsSync)(entryPointPath)) {
                return entryPointPath;
            }
            else {
                logger.warn(`Could not determine the entry point for "${packageJsonPath}". Package will be ignored.`);
                logger.verbose(e.message);
                return exports.ignorePackage;
            }
        }
    }
    return getTsSourceFromJsSource(logger, entryPointPath);
}
exports.getTsEntryPointForPackage = getTsEntryPointForPackage;
