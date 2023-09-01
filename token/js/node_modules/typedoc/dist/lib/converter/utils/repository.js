"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.guessSourceUrlTemplate = exports.Repository = exports.gitIsInstalled = void 0;
const child_process_1 = require("child_process");
const base_path_1 = require("../utils/base-path");
const TEN_MEGABYTES = 1024 * 10000;
function git(...args) {
    return (0, child_process_1.spawnSync)("git", args, {
        encoding: "utf-8",
        windowsHide: true,
        maxBuffer: TEN_MEGABYTES,
    });
}
exports.gitIsInstalled = git("--version").status === 0;
/**
 * Stores data of a repository.
 */
class Repository {
    /**
     * Create a new Repository instance.
     *
     * @param path  The root path of the repository.
     */
    constructor(path, gitRevision, urlTemplate) {
        /**
         * All files tracked by the repository.
         */
        this.files = new Set();
        this.path = path;
        this.gitRevision = gitRevision;
        this.urlTemplate = urlTemplate;
        const out = git("-C", path, "ls-files");
        if (out.status === 0) {
            out.stdout.split("\n").forEach((file) => {
                if (file !== "") {
                    this.files.add(base_path_1.BasePath.normalize(path + "/" + file));
                }
            });
        }
    }
    /**
     * Get the URL of the given file on GitHub or Bitbucket.
     *
     * @param fileName  The file whose URL should be determined.
     * @returns A URL pointing to the web preview of the given file or undefined.
     */
    getURL(fileName, line) {
        if (!this.files.has(fileName)) {
            return;
        }
        const replacements = {
            gitRevision: this.gitRevision,
            path: fileName.substring(this.path.length + 1),
            line,
        };
        return this.urlTemplate.replace(/\{(gitRevision|path|line)\}/g, (_, key) => replacements[key]);
    }
    /**
     * Try to create a new repository instance.
     *
     * Checks whether the given path is the root of a valid repository and if so
     * creates a new instance of {@link Repository}.
     *
     * @param path  The potential repository root.
     * @returns A new instance of {@link Repository} or undefined.
     */
    static tryCreateRepository(path, sourceLinkTemplate, gitRevision, gitRemote, logger) {
        const topLevel = git("-C", path, "rev-parse", "--show-toplevel");
        if (topLevel.status !== 0)
            return;
        gitRevision || (gitRevision = git("-C", path, "rev-parse", "--short", "HEAD").stdout.trim());
        if (!gitRevision)
            return; // Will only happen in a repo with no commits.
        let urlTemplate;
        if (sourceLinkTemplate) {
            urlTemplate = sourceLinkTemplate;
        }
        else if (/^https?:\/\//.test(gitRemote)) {
            logger.warn("Using a link as the gitRemote is deprecated and will be removed in 0.24.");
            urlTemplate = `${gitRemote}/{gitRevision}`;
        }
        else {
            const remotesOut = git("-C", path, "remote", "get-url", gitRemote);
            if (remotesOut.status === 0) {
                urlTemplate = guessSourceUrlTemplate(remotesOut.stdout.split("\n"));
            }
            else {
                logger.warn(`The provided git remote "${gitRemote}" was not valid. Source links will be broken.`);
            }
        }
        if (!urlTemplate)
            return;
        return new Repository(base_path_1.BasePath.normalize(topLevel.stdout.replace("\n", "")), gitRevision, urlTemplate);
    }
}
exports.Repository = Repository;
// Should have three capturing groups:
// 1. hostname
// 2. user
// 3. project
const repoExpressions = [
    /(github(?!.us)(?:\.[a-z]+)*\.[a-z]{2,})[:/]([^/]+)\/(.*)/,
    /(\w+\.githubprivate.com)[:/]([^/]+)\/(.*)/,
    /(\w+\.ghe.com)[:/]([^/]+)\/(.*)/,
    /(\w+\.github.us)[:/]([^/]+)\/(.*)/,
    /(bitbucket.org)[:/]([^/]+)\/(.*)/,
    /(gitlab.com)[:/]([^/]+)\/(.*)/,
];
function guessSourceUrlTemplate(remotes) {
    let hostname = "";
    let user = "";
    let project = "";
    outer: for (const repoLink of remotes) {
        for (const regex of repoExpressions) {
            const match = regex.exec(repoLink);
            if (match) {
                hostname = match[1];
                user = match[2];
                project = match[3];
                break outer;
            }
        }
    }
    if (!hostname)
        return;
    if (project.endsWith(".git")) {
        project = project.slice(0, -4);
    }
    let sourcePath = "blob";
    let anchorPrefix = "L";
    if (hostname.includes("gitlab")) {
        sourcePath = "-/blob";
    }
    else if (hostname.includes("bitbucket")) {
        sourcePath = "src";
        anchorPrefix = "lines-";
    }
    return `https://${hostname}/${user}/${project}/${sourcePath}/{gitRevision}/{path}#${anchorPrefix}{line}`;
}
exports.guessSourceUrlTemplate = guessSourceUrlTemplate;
