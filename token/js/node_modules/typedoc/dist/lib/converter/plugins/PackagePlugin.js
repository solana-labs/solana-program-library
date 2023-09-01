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
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.PackagePlugin = void 0;
const Path = __importStar(require("path"));
const components_1 = require("../components");
const converter_1 = require("../converter");
const utils_1 = require("../../utils");
const fs_1 = require("../../utils/fs");
const paths_1 = require("../../utils/paths");
const minimalSourceFile_1 = require("../../utils/minimalSourceFile");
const application_events_1 = require("../../application-events");
const path_1 = require("path");
/**
 * A handler that tries to find the package.json and readme.md files of the
 * current project.
 */
let PackagePlugin = class PackagePlugin extends components_1.ConverterComponent {
    initialize() {
        this.listenTo(this.owner, {
            [converter_1.Converter.EVENT_BEGIN]: this.onBegin,
            [converter_1.Converter.EVENT_RESOLVE_BEGIN]: this.onBeginResolve,
            [converter_1.Converter.EVENT_END]: () => {
                delete this.readmeFile;
                delete this.readmeContents;
                delete this.packageJson;
            },
        });
        this.listenTo(this.application, {
            [application_events_1.ApplicationEvents.REVIVE]: this.onRevive,
        });
    }
    onRevive(project) {
        this.onBegin();
        this.addEntries(project);
        delete this.readmeFile;
        delete this.packageJson;
        delete this.readmeContents;
    }
    onBegin() {
        this.readmeFile = undefined;
        this.readmeContents = undefined;
        this.packageJson = undefined;
        const entryFiles = this.entryPointStrategy === utils_1.EntryPointStrategy.Packages
            ? this.entryPoints.map((d) => (0, path_1.join)(d, "package.json"))
            : this.entryPoints;
        const dirName = this.application.options.packageDir ??
            Path.resolve((0, fs_1.deriveRootDir)(entryFiles));
        this.application.logger.verbose(`Begin readme.md/package.json search at ${(0, paths_1.nicePath)(dirName)}`);
        this.packageJson = (0, fs_1.discoverPackageJson)(dirName)?.content;
        // Path will be resolved already. This is kind of ugly, but...
        if (this.readme.endsWith("none")) {
            return; // No readme, we're done
        }
        if (this.readme) {
            // Readme path provided, read only that file.
            try {
                this.readmeContents = (0, utils_1.readFile)(this.readme);
                this.readmeFile = this.readme;
            }
            catch {
                this.application.logger.error(`Provided README path, ${(0, paths_1.nicePath)(this.readme)} could not be read.`);
            }
        }
        else {
            // No readme provided, automatically find the readme
            const result = (0, fs_1.discoverInParentDir)("readme.md", dirName, (content) => content);
            if (result) {
                this.readmeFile = result.file;
                this.readmeContents = result.content;
            }
        }
    }
    onBeginResolve(context) {
        this.addEntries(context.project);
    }
    addEntries(project) {
        if (this.readmeFile && this.readmeContents) {
            const comment = this.application.converter.parseRawComment(new minimalSourceFile_1.MinimalSourceFile(this.readmeContents, this.readmeFile));
            if (comment.blockTags.length || comment.modifierTags.size) {
                const ignored = [
                    ...comment.blockTags.map((tag) => tag.tag),
                    ...comment.modifierTags,
                ];
                this.application.logger.warn(`Block and modifier tags will be ignored within the readme:\n\t${ignored.join("\n\t")}`);
            }
            project.readme = comment.summary;
        }
        if (this.packageJson) {
            project.packageName = this.packageJson.name;
            if (!project.name) {
                project.name = project.packageName || "Documentation";
            }
            if (this.includeVersion) {
                project.packageVersion = this.packageJson.version?.replace(/^v/, "");
            }
        }
        else if (!project.name) {
            this.application.logger.warn('The --name option was not specified, and no package.json was found. Defaulting project name to "Documentation".');
            project.name = "Documentation";
        }
    }
};
__decorate([
    (0, utils_1.BindOption)("readme")
], PackagePlugin.prototype, "readme", void 0);
__decorate([
    (0, utils_1.BindOption)("entryPointStrategy")
], PackagePlugin.prototype, "entryPointStrategy", void 0);
__decorate([
    (0, utils_1.BindOption)("entryPoints")
], PackagePlugin.prototype, "entryPoints", void 0);
__decorate([
    (0, utils_1.BindOption)("includeVersion")
], PackagePlugin.prototype, "includeVersion", void 0);
PackagePlugin = __decorate([
    (0, components_1.Component)({ name: "package" })
], PackagePlugin);
exports.PackagePlugin = PackagePlugin;
