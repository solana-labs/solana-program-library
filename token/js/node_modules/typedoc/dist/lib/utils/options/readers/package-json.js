"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.PackageJsonReader = void 0;
const assert_1 = require("assert");
const paths_1 = require("../../paths");
const fs_1 = require("../../fs");
const path_1 = require("path");
class PackageJsonReader {
    constructor() {
        // Should run after the TypeDoc config reader but before the TS config
        // reader, so that it can still specify a path to a `tsconfig.json` file.
        this.order = 150;
        this.supportsPackages = true;
        this.name = "package-json";
    }
    read(container, logger, cwd) {
        const result = (0, fs_1.discoverPackageJson)(cwd);
        if (!result) {
            return;
        }
        const { file, content } = result;
        if ("typedoc" in content) {
            logger.warn(`The 'typedoc' key in ${(0, paths_1.nicePath)(file)} was used by the legacy-packages entryPointStrategy and will be ignored.`);
        }
        const optsKey = "typedocOptions";
        if (!(optsKey in content)) {
            return;
        }
        const opts = content[optsKey];
        if (opts === null || typeof opts !== "object") {
            logger.error(`Failed to parse the "typedocOptions" field in ${(0, paths_1.nicePath)(file)}, ensure it exists and contains an object.`);
            return;
        }
        for (const [opt, val] of Object.entries(opts)) {
            try {
                container.setValue(opt, val, (0, path_1.dirname)(file));
            }
            catch (err) {
                (0, assert_1.ok)(err instanceof Error);
                logger.error(err.message);
            }
        }
    }
}
exports.PackageJsonReader = PackageJsonReader;
