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
exports.findPackageForPath = exports.discoverPackageJson = exports.discoverInParentDirExactMatch = exports.discoverInParentDir = exports.hasTsExtension = exports.glob = exports.copySync = exports.copy = exports.writeFile = exports.writeFileSync = exports.readFile = exports.getCommonDirectory = exports.deriveRootDir = exports.isDir = exports.isFile = void 0;
const fs = __importStar(require("fs"));
const fs_1 = require("fs");
const minimatch_1 = require("minimatch");
const path_1 = require("path");
const validation_1 = require("./validation");
const paths_1 = require("./paths");
const array_1 = require("./array");
function isFile(file) {
    try {
        return fs.statSync(file).isFile();
    }
    catch {
        return false;
    }
}
exports.isFile = isFile;
function isDir(path) {
    try {
        return fs.statSync(path).isDirectory();
    }
    catch {
        return false;
    }
}
exports.isDir = isDir;
function deriveRootDir(globPaths) {
    const normalized = globPaths.map(paths_1.normalizePath);
    const globs = (0, paths_1.createMinimatch)(normalized);
    const rootPaths = globs.flatMap((glob, i) => (0, array_1.filterMap)(glob.set, (set) => {
        const stop = set.findIndex((part) => typeof part !== "string");
        if (stop === -1) {
            return normalized[i];
        }
        else {
            const kept = set.slice(0, stop).join("/");
            return normalized[i].substring(0, normalized[i].indexOf(kept) + kept.length);
        }
    }));
    return getCommonDirectory(rootPaths);
}
exports.deriveRootDir = deriveRootDir;
/**
 * Get the longest directory path common to all files.
 */
function getCommonDirectory(files) {
    if (!files.length) {
        return "";
    }
    const roots = files.map((f) => f.split(/\\|\//));
    if (roots.length === 1) {
        return roots[0].slice(0, -1).join("/");
    }
    let i = 0;
    while (i < roots[0].length &&
        new Set(roots.map((part) => part[i])).size === 1) {
        i++;
    }
    return roots[0].slice(0, i).join("/");
}
exports.getCommonDirectory = getCommonDirectory;
/**
 * Load the given file and return its contents.
 *
 * @param file  The path of the file to read.
 * @returns The files contents.
 */
function readFile(file) {
    const buffer = fs.readFileSync(file);
    switch (buffer[0]) {
        case 0xfe:
            if (buffer[1] === 0xff) {
                let i = 0;
                while (i + 1 < buffer.length) {
                    const temp = buffer[i];
                    buffer[i] = buffer[i + 1];
                    buffer[i + 1] = temp;
                    i += 2;
                }
                return buffer.toString("ucs2", 2);
            }
            break;
        case 0xff:
            if (buffer[1] === 0xfe) {
                return buffer.toString("ucs2", 2);
            }
            break;
        case 0xef:
            if (buffer[1] === 0xbb) {
                return buffer.toString("utf8", 3);
            }
    }
    return buffer.toString("utf8", 0);
}
exports.readFile = readFile;
/**
 * Write a file to disc.
 *
 * If the containing directory does not exist it will be created.
 *
 * @param fileName  The name of the file that should be written.
 * @param data  The contents of the file.
 */
function writeFileSync(fileName, data) {
    fs.mkdirSync((0, path_1.dirname)((0, paths_1.normalizePath)(fileName)), { recursive: true });
    fs.writeFileSync((0, paths_1.normalizePath)(fileName), data);
}
exports.writeFileSync = writeFileSync;
/**
 * Write a file to disc.
 *
 * If the containing directory does not exist it will be created.
 *
 * @param fileName  The name of the file that should be written.
 * @param data  The contents of the file.
 */
async function writeFile(fileName, data) {
    await fs_1.promises.mkdir((0, path_1.dirname)((0, paths_1.normalizePath)(fileName)), {
        recursive: true,
    });
    await fs_1.promises.writeFile((0, paths_1.normalizePath)(fileName), data);
}
exports.writeFile = writeFile;
/**
 * Copy a file or directory recursively.
 */
async function copy(src, dest) {
    const stat = await fs_1.promises.stat(src);
    if (stat.isDirectory()) {
        const contained = await fs_1.promises.readdir(src);
        await Promise.all(contained.map((file) => copy((0, path_1.join)(src, file), (0, path_1.join)(dest, file))));
    }
    else if (stat.isFile()) {
        await fs_1.promises.mkdir((0, path_1.dirname)(dest), { recursive: true });
        await fs_1.promises.copyFile(src, dest);
    }
    else {
        // Do nothing for FIFO, special devices.
    }
}
exports.copy = copy;
function copySync(src, dest) {
    const stat = fs.statSync(src);
    if (stat.isDirectory()) {
        const contained = fs.readdirSync(src);
        contained.forEach((file) => copySync((0, path_1.join)(src, file), (0, path_1.join)(dest, file)));
    }
    else if (stat.isFile()) {
        fs.mkdirSync((0, path_1.dirname)(dest), { recursive: true });
        fs.copyFileSync(src, dest);
    }
    else {
        // Do nothing for FIFO, special devices.
    }
}
exports.copySync = copySync;
/**
 * Simpler version of `glob.sync` that only covers our use cases, always ignoring node_modules.
 */
function glob(pattern, root, options = {}) {
    const result = [];
    const mini = new minimatch_1.Minimatch((0, paths_1.normalizePath)(pattern));
    const dirs = [(0, paths_1.normalizePath)(root).split("/")];
    // cache of real paths to avoid infinite recursion
    const symlinkTargetsSeen = new Set();
    // cache of fs.realpathSync results to avoid extra I/O
    const realpathCache = new Map();
    const { includeDirectories = false, followSymlinks = false } = options;
    // if we _specifically asked_ for something in node_modules, fine, otherwise ignore it
    // to avoid globs like '**/*.ts' finding all the .d.ts files in node_modules.
    // however, if the pattern is something like `!**/node_modules/**`, this will also
    // cause node_modules to be considered, though it will be discarded by minimatch.
    const shouldIncludeNodeModules = pattern.includes("node_modules");
    let dir = dirs.shift();
    const handleFile = (path) => {
        const childPath = [...dir, path].join("/");
        if (mini.match(childPath)) {
            result.push(childPath);
        }
    };
    const handleDirectory = (path) => {
        const childPath = [...dir, path];
        if (mini.set.some((row) => mini.matchOne(childPath, row, /* partial */ true))) {
            dirs.push(childPath);
        }
    };
    const handleSymlink = (path) => {
        const childPath = [...dir, path].join("/");
        let realpath;
        try {
            realpath =
                realpathCache.get(childPath) ?? fs.realpathSync(childPath);
            realpathCache.set(childPath, realpath);
        }
        catch {
            return;
        }
        if (symlinkTargetsSeen.has(realpath)) {
            return;
        }
        symlinkTargetsSeen.add(realpath);
        try {
            const stats = fs.statSync(realpath);
            if (stats.isDirectory()) {
                handleDirectory(path);
            }
            else if (stats.isFile()) {
                handleFile(path);
            }
            else if (stats.isSymbolicLink()) {
                const dirpath = dir.join("/");
                if (dirpath === realpath) {
                    // special case: real path of symlink is the directory we're currently traversing
                    return;
                }
                const targetPath = (0, path_1.relative)(dirpath, realpath);
                handleSymlink(targetPath);
            } // everything else should be ignored
        }
        catch (e) {
            // invalid symbolic link; ignore
        }
    };
    while (dir) {
        if (includeDirectories && mini.match(dir.join("/"))) {
            result.push(dir.join("/"));
        }
        for (const child of fs.readdirSync(dir.join("/"), {
            withFileTypes: true,
        })) {
            if (child.isFile()) {
                handleFile(child.name);
            }
            else if (child.isDirectory()) {
                if (shouldIncludeNodeModules || child.name !== "node_modules") {
                    handleDirectory(child.name);
                }
            }
            else if (followSymlinks && child.isSymbolicLink()) {
                handleSymlink(child.name);
            }
        }
        dir = dirs.shift();
    }
    return result;
}
exports.glob = glob;
function hasTsExtension(path) {
    return /\.[cm]?ts$|\.tsx$/.test(path);
}
exports.hasTsExtension = hasTsExtension;
function discoverInParentDir(name, dir, read) {
    if (!isDir(dir))
        return;
    const reachedTopDirectory = (dirName) => dirName === (0, path_1.resolve)((0, path_1.join)(dirName, ".."));
    while (!reachedTopDirectory(dir)) {
        for (const file of fs.readdirSync(dir)) {
            if (file.toLowerCase() !== name.toLowerCase())
                continue;
            try {
                const content = read(readFile((0, path_1.join)(dir, file)));
                if (content != null) {
                    return { file: (0, path_1.join)(dir, file), content };
                }
            }
            catch {
                // Ignore, file didn't pass validation
            }
        }
        dir = (0, path_1.resolve)((0, path_1.join)(dir, ".."));
    }
}
exports.discoverInParentDir = discoverInParentDir;
function discoverInParentDirExactMatch(name, dir, read) {
    if (!isDir(dir))
        return;
    const reachedTopDirectory = (dirName) => dirName === (0, path_1.resolve)((0, path_1.join)(dirName, ".."));
    while (!reachedTopDirectory(dir)) {
        try {
            const content = read(readFile((0, path_1.join)(dir, name)));
            if (content != null) {
                return { file: (0, path_1.join)(dir, name), content };
            }
        }
        catch {
            // Ignore, file didn't pass validation
        }
        dir = (0, path_1.resolve)((0, path_1.join)(dir, ".."));
    }
}
exports.discoverInParentDirExactMatch = discoverInParentDirExactMatch;
function discoverPackageJson(dir) {
    return discoverInParentDirExactMatch("package.json", dir, (content) => {
        const pkg = JSON.parse(content);
        if ((0, validation_1.validate)({ name: String, version: (0, validation_1.optional)(String) }, pkg)) {
            return pkg;
        }
    });
}
exports.discoverPackageJson = discoverPackageJson;
// dir -> package name according to package.json in this or some parent dir
const packageCache = new Map();
function findPackageForPath(sourcePath) {
    const dir = (0, path_1.dirname)(sourcePath);
    const cache = packageCache.get(dir);
    if (cache) {
        return cache;
    }
    const packageJson = discoverPackageJson(dir);
    if (packageJson) {
        packageCache.set(dir, packageJson.content.name);
        return packageJson.content.name;
    }
}
exports.findPackageForPath = findPackageForPath;
