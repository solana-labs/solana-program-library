"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.normalizePath = exports.nicePath = exports.matchesAny = exports.createMinimatch = void 0;
const minimatch_1 = require("minimatch");
const path_1 = require("path");
/**
 * Convert array of glob patterns to array of minimatch instances.
 *
 * Handle a few Windows-Unix path gotchas.
 */
function createMinimatch(patterns) {
    return patterns.map((pattern) => new minimatch_1.Minimatch(normalizePath(pattern).replace(/^\w:\//, ""), {
        dot: true,
    }));
}
exports.createMinimatch = createMinimatch;
function matchesAny(patterns, path) {
    const normPath = normalizePath(path).replace(/^\w:\//, "");
    return patterns.some((pat) => pat.match(normPath));
}
exports.matchesAny = matchesAny;
function nicePath(absPath) {
    if (!(0, path_1.isAbsolute)(absPath))
        return absPath;
    const relativePath = (0, path_1.relative)(process.cwd(), absPath);
    if (relativePath.startsWith("..")) {
        return normalizePath(absPath);
    }
    return `./${normalizePath(relativePath)}`;
}
exports.nicePath = nicePath;
/**
 * Normalize the given path.
 *
 * @param path  The path that should be normalized.
 * @returns The normalized path.
 */
function normalizePath(path) {
    return path.replace(/\\/g, "/");
}
exports.normalizePath = normalizePath;
