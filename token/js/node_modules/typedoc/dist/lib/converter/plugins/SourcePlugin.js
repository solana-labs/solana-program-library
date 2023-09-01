"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.SourcePlugin = void 0;
const typescript_1 = __importDefault(require("typescript"));
const index_1 = require("../../models/reflections/index");
const components_1 = require("../components");
const converter_1 = require("../converter");
const utils_1 = require("../../utils");
const nodes_1 = require("../utils/nodes");
const path_1 = require("path");
const models_1 = require("../../models");
const repository_1 = require("../utils/repository");
const base_path_1 = require("../utils/base-path");
/**
 * A handler that attaches source file information to reflections.
 */
let SourcePlugin = class SourcePlugin extends components_1.ConverterComponent {
    constructor() {
        super(...arguments);
        /**
         * All file names to find the base path from.
         */
        this.fileNames = new Set();
        /**
         * List of known repositories.
         */
        this.repositories = {};
        /**
         * List of paths known to be not under git control.
         */
        this.ignoredPaths = new Set();
    }
    /**
     * Create a new SourceHandler instance.
     */
    initialize() {
        this.listenTo(this.owner, {
            [converter_1.Converter.EVENT_END]: this.onEnd,
            [converter_1.Converter.EVENT_CREATE_DECLARATION]: this.onDeclaration,
            [converter_1.Converter.EVENT_CREATE_SIGNATURE]: this.onSignature,
            [converter_1.Converter.EVENT_RESOLVE_BEGIN]: this.onBeginResolve,
        });
    }
    onEnd() {
        // Should probably clear repositories/ignoredPaths here, but these aren't likely to change between runs...
        this.fileNames.clear();
    }
    /**
     * Triggered when the converter has created a declaration reflection.
     *
     * Attach the current source file to the {@link DeclarationReflection.sources} array.
     *
     * @param _context  The context object describing the current state the converter is in.
     * @param reflection  The reflection that is currently processed.
     */
    onDeclaration(_context, reflection) {
        if (this.disableSources)
            return;
        const symbol = reflection.project.getSymbolFromReflection(reflection);
        for (const node of symbol?.declarations || []) {
            const sourceFile = node.getSourceFile();
            const fileName = base_path_1.BasePath.normalize(sourceFile.fileName);
            this.fileNames.add(fileName);
            let position;
            if ((0, nodes_1.isNamedNode)(node)) {
                position = typescript_1.default.getLineAndCharacterOfPosition(sourceFile, node.name.getStart());
            }
            else if (typescript_1.default.isSourceFile(node)) {
                position = { character: 0, line: 0 };
            }
            else {
                position = typescript_1.default.getLineAndCharacterOfPosition(sourceFile, node.getStart());
            }
            reflection.sources || (reflection.sources = []);
            reflection.sources.push(new models_1.SourceReference(fileName, position.line + 1, position.character));
        }
    }
    onSignature(_context, reflection, sig) {
        if (this.disableSources || !sig)
            return;
        const sourceFile = sig.getSourceFile();
        const fileName = base_path_1.BasePath.normalize(sourceFile.fileName);
        this.fileNames.add(fileName);
        const position = typescript_1.default.getLineAndCharacterOfPosition(sourceFile, sig.getStart());
        reflection.sources || (reflection.sources = []);
        reflection.sources.push(new models_1.SourceReference(fileName, position.line + 1, position.character));
    }
    /**
     * Triggered when the converter begins resolving a project.
     *
     * @param context  The context object describing the current state the converter is in.
     */
    onBeginResolve(context) {
        if (this.disableSources)
            return;
        const basePath = this.basePath || (0, utils_1.getCommonDirectory)([...this.fileNames]);
        for (const refl of Object.values(context.project.reflections)) {
            if (!(refl instanceof index_1.DeclarationReflection ||
                refl instanceof index_1.SignatureReflection)) {
                continue;
            }
            for (const source of refl.sources || []) {
                if (repository_1.gitIsInstalled) {
                    const repo = this.getRepository(source.fullFileName);
                    source.url = repo?.getURL(source.fullFileName, source.line);
                }
                source.fileName = (0, utils_1.normalizePath)((0, path_1.relative)(basePath, source.fullFileName));
            }
        }
    }
    /**
     * Check whether the given file is placed inside a repository.
     *
     * @param fileName  The name of the file a repository should be looked for.
     * @returns The found repository info or undefined.
     */
    getRepository(fileName) {
        // Check for known non-repositories
        const dirName = (0, path_1.dirname)(fileName);
        const segments = dirName.split("/");
        for (let i = segments.length; i > 0; i--) {
            if (this.ignoredPaths.has(segments.slice(0, i).join("/"))) {
                return;
            }
        }
        // Check for known repositories
        for (const path of Object.keys(this.repositories)) {
            if (fileName.toLowerCase().startsWith(path)) {
                return this.repositories[path];
            }
        }
        // Try to create a new repository
        const repository = repository_1.Repository.tryCreateRepository(dirName, this.sourceLinkTemplate, this.gitRevision, this.gitRemote, this.application.logger);
        if (repository) {
            this.repositories[repository.path.toLowerCase()] = repository;
            return repository;
        }
        // No repository found, add path to ignored paths
        this.ignoredPaths.add(dirName);
    }
};
__decorate([
    (0, utils_1.BindOption)("disableSources")
], SourcePlugin.prototype, "disableSources", void 0);
__decorate([
    (0, utils_1.BindOption)("gitRevision")
], SourcePlugin.prototype, "gitRevision", void 0);
__decorate([
    (0, utils_1.BindOption)("gitRemote")
], SourcePlugin.prototype, "gitRemote", void 0);
__decorate([
    (0, utils_1.BindOption)("sourceLinkTemplate")
], SourcePlugin.prototype, "sourceLinkTemplate", void 0);
__decorate([
    (0, utils_1.BindOption)("basePath")
], SourcePlugin.prototype, "basePath", void 0);
SourcePlugin = __decorate([
    (0, components_1.Component)({ name: "source" })
], SourcePlugin);
exports.SourcePlugin = SourcePlugin;
