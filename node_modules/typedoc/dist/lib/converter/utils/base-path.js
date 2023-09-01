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
exports.BasePath = void 0;
const Path = __importStar(require("path"));
/**
 * Helper class that determines the common base path of a set of files.
 *
 * In the first step all files must be passed to {@link add}. Afterwards {@link trim}
 * can be used to retrieve the shortest path relative to the determined base path.
 */
class BasePath {
    constructor() {
        /**
         * List of known base paths.
         */
        this.basePaths = [];
    }
    /**
     * Add the given file path to this set of base paths.
     *
     * @param fileName  The absolute filename that should be added to the base path.
     */
    add(fileName) {
        const fileDir = Path.dirname(BasePath.normalize(fileName));
        const filePath = fileDir.split("/");
        basePaths: for (let n = 0, c = this.basePaths.length; n < c; n++) {
            const basePath = this.basePaths[n].split("/");
            const mMax = Math.min(basePath.length, filePath.length);
            for (let m = 0; m < mMax; m++) {
                if (basePath[m] === filePath[m]) {
                    continue;
                }
                if (m < 1) {
                    // No match at all, try next known base path
                    continue basePaths;
                }
                else {
                    // Partial match, trim the known base path
                    if (m < basePath.length) {
                        this.basePaths[n] = basePath.slice(0, m).join("/");
                    }
                    return;
                }
            }
            // Complete match, exit
            this.basePaths[n] = basePath.splice(0, mMax).join("/");
            return;
        }
        // Unknown base path, add it
        this.basePaths.push(fileDir);
    }
    /**
     * Trim the given filename by the determined base paths.
     *
     * @param fileName  The absolute filename that should be trimmed.
     * @returns The trimmed version of the filename.
     */
    trim(fileName) {
        fileName = BasePath.normalize(fileName);
        for (let n = 0, c = this.basePaths.length; n < c; n++) {
            const basePath = this.basePaths[n];
            if (fileName.substring(0, basePath.length) === basePath) {
                return fileName.substring(basePath.length + 1);
            }
        }
        return fileName;
    }
    /**
     * Reset this instance, ignore all paths already passed to {@link add}.
     */
    reset() {
        this.basePaths = [];
    }
    /**
     * Normalize the given path.
     *
     * @param path  The path that should be normalized.
     * @returns Normalized version of the given path.
     */
    static normalize(path) {
        // Ensure forward slashes
        path = path.replace(/\\/g, "/");
        // Remove all surrounding quotes
        path = path.replace(/^["']+|["']+$/g, "");
        // Make Windows drive letters upper case
        return path.replace(/^([^:]+):\//, (_m, m1) => m1.toUpperCase() + ":/");
    }
}
exports.BasePath = BasePath;
