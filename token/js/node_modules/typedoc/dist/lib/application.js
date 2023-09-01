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
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
var Application_1;
Object.defineProperty(exports, "__esModule", { value: true });
exports.Application = void 0;
const Path = __importStar(require("path"));
const typescript_1 = __importDefault(require("typescript"));
const index_1 = require("./converter/index");
const renderer_1 = require("./output/renderer");
const serialization_1 = require("./serialization");
const index_2 = require("./utils/index");
const component_1 = require("./utils/component");
const utils_1 = require("./utils");
const array_1 = require("./utils/array");
const assert_1 = require("assert");
const entry_point_1 = require("./utils/entry-point");
const paths_1 = require("./utils/paths");
const general_1 = require("./utils/general");
const exports_1 = require("./validation/exports");
const documentation_1 = require("./validation/documentation");
const links_1 = require("./validation/links");
const application_events_1 = require("./application-events");
const tsconfig_1 = require("./utils/tsconfig");
const fs_1 = require("./utils/fs");
const abstract_1 = require("./models/reflections/abstract");
// eslint-disable-next-line @typescript-eslint/no-var-requires
const packageInfo = require("../../package.json");
const supportedVersionMajorMinor = packageInfo.peerDependencies.typescript
    .split("||")
    .map((version) => version.replace(/^\s*|\.x\s*$/g, ""));
/**
 * The default TypeDoc main application class.
 *
 * This class holds the two main components of TypeDoc, the {@link Converter} and
 * the {@link Renderer}. When running TypeDoc, first the {@link Converter} is invoked which
 * generates a {@link ProjectReflection} from the passed in source files. The
 * {@link ProjectReflection} is a hierarchical model representation of the TypeScript
 * project. Afterwards the model is passed to the {@link Renderer} which uses an instance
 * of {@link Theme} to generate the final documentation.
 *
 * Both the {@link Converter} and the {@link Renderer} emit a series of events while processing the project.
 * Subscribe to these Events to control the application flow or alter the output.
 */
let Application = Application_1 = class Application extends component_1.ChildableComponent {
    /**
     * Create a new TypeDoc application instance.
     */
    constructor() {
        super(null); // We own ourselves
        /**
         * The serializer used to generate JSON output.
         */
        this.serializer = new serialization_1.Serializer();
        /**
         * The deserializer used to restore previously serialized JSON output.
         */
        this.deserializer = new serialization_1.Deserializer(this);
        this.logger = new index_2.ConsoleLogger();
        this.options = new utils_1.Options(this.logger);
        this.converter = this.addComponent("converter", index_1.Converter);
        this.renderer = this.addComponent("renderer", renderer_1.Renderer);
    }
    /**
     * Initialize TypeDoc, loading plugins if applicable.
     */
    async bootstrapWithPlugins(options = {}) {
        this.options.reset();
        this.setOptions(options, /* reportErrors */ false);
        this.options.read(new index_2.Logger());
        this.logger.level = this.options.getValue("logLevel");
        await (0, index_2.loadPlugins)(this, this.options.getValue("plugin"));
        this.bootstrap(options);
    }
    /**
     * Initialize TypeDoc without loading plugins.
     */
    bootstrap(options = {}) {
        this.options.reset();
        this.setOptions(options, /* reportErrors */ false);
        this.options.read(this.logger);
        this.setOptions(options);
        this.logger.level = this.options.getValue("logLevel");
        if ((0, general_1.hasBeenLoadedMultipleTimes)()) {
            this.logger.warn(`TypeDoc has been loaded multiple times. This is commonly caused by plugins which have their own installation of TypeDoc. The loaded paths are:\n\t${(0, general_1.getLoadedPaths)().join("\n\t")}`);
        }
        this.trigger(application_events_1.ApplicationEvents.BOOTSTRAP_END, this);
    }
    setOptions(options, reportErrors = true) {
        for (const [key, val] of Object.entries(options)) {
            try {
                this.options.setValue(key, val);
            }
            catch (error) {
                (0, assert_1.ok)(error instanceof Error);
                if (reportErrors) {
                    this.logger.error(error.message);
                }
            }
        }
    }
    /**
     * Return the path to the TypeScript compiler.
     */
    getTypeScriptPath() {
        return (0, paths_1.nicePath)(Path.dirname(require.resolve("typescript")));
    }
    getTypeScriptVersion() {
        return typescript_1.default.version;
    }
    /**
     * Gets the entry points to be documented according to the current `entryPoints` and `entryPointStrategy` options.
     * May return undefined if entry points fail to be expanded.
     */
    getEntryPoints() {
        return (0, entry_point_1.getEntryPoints)(this.logger, this.options);
    }
    /**
     * Run the converter for the given set of files and return the generated reflections.
     *
     * @returns An instance of ProjectReflection on success, undefined otherwise.
     */
    convert() {
        const start = Date.now();
        // We freeze here rather than in the Converter class since TypeDoc's tests reuse the Application
        // with a few different settings.
        this.options.freeze();
        this.logger.verbose(`Using TypeScript ${this.getTypeScriptVersion()} from ${this.getTypeScriptPath()}`);
        if (this.entryPointStrategy === entry_point_1.EntryPointStrategy.Merge) {
            return this._merge();
        }
        if (this.entryPointStrategy === entry_point_1.EntryPointStrategy.Packages) {
            return this._convertPackages();
        }
        if (!supportedVersionMajorMinor.some((version) => version == typescript_1.default.versionMajorMinor)) {
            this.logger.warn(`You are running with an unsupported TypeScript version! If TypeDoc crashes, this is why. TypeDoc supports ${supportedVersionMajorMinor.join(", ")}`);
        }
        const entryPoints = this.getEntryPoints();
        if (!entryPoints) {
            // Fatal error already reported.
            return;
        }
        const programs = (0, array_1.unique)(entryPoints.map((e) => e.program));
        this.logger.verbose(`Converting with ${programs.length} programs ${entryPoints.length} entry points`);
        if (this.skipErrorChecking === false) {
            const errors = programs.flatMap((program) => typescript_1.default.getPreEmitDiagnostics(program));
            if (errors.length) {
                this.logger.diagnostics(errors);
                return;
            }
        }
        if (this.options.getValue("emit") === "both") {
            for (const program of programs) {
                program.emit();
            }
        }
        const startConversion = Date.now();
        this.logger.verbose(`Finished getting entry points in ${Date.now() - start}ms`);
        const project = this.converter.convert(entryPoints);
        this.logger.verbose(`Finished conversion in ${Date.now() - startConversion}ms`);
        return project;
    }
    convertAndWatch(success) {
        this.options.freeze();
        if (!this.options.getValue("preserveWatchOutput") &&
            this.logger instanceof index_2.ConsoleLogger) {
            typescript_1.default.sys.clearScreen?.();
        }
        this.logger.verbose(`Using TypeScript ${this.getTypeScriptVersion()} from ${this.getTypeScriptPath()}`);
        if (!supportedVersionMajorMinor.some((version) => version == typescript_1.default.versionMajorMinor)) {
            this.logger.warn(`You are running with an unsupported TypeScript version! TypeDoc supports ${supportedVersionMajorMinor.join(", ")}`);
        }
        if (Object.keys(this.options.getCompilerOptions()).length === 0) {
            this.logger.warn(`No compiler options set. This likely means that TypeDoc did not find your tsconfig.json. Generated documentation will probably be empty.`);
        }
        // Doing this is considerably more complicated, we'd need to manage an array of programs, not convert until all programs
        // have reported in the first time... just error out for now. I'm not convinced anyone will actually notice.
        if (this.options.getFileNames().length === 0) {
            this.logger.error("The provided tsconfig file looks like a solution style tsconfig, which is not supported in watch mode.");
            return;
        }
        // Support for packages mode is currently unimplemented
        if (this.entryPointStrategy !== entry_point_1.EntryPointStrategy.Resolve &&
            this.entryPointStrategy !== entry_point_1.EntryPointStrategy.Expand) {
            this.logger.error("entryPointStrategy must be set to either resolve or expand for watch mode.");
            return;
        }
        const tsconfigFile = (0, tsconfig_1.findTsConfigFile)(this.options.getValue("tsconfig")) ??
            "tsconfig.json";
        // We don't want to do it the first time to preserve initial debug status messages. They'll be lost
        // after the user saves a file, but better than nothing...
        let firstStatusReport = true;
        const host = typescript_1.default.createWatchCompilerHost(tsconfigFile, {}, typescript_1.default.sys, typescript_1.default.createEmitAndSemanticDiagnosticsBuilderProgram, (diagnostic) => this.logger.diagnostic(diagnostic), (status, newLine, _options, errorCount) => {
            if (!firstStatusReport &&
                errorCount === void 0 &&
                !this.options.getValue("preserveWatchOutput") &&
                this.logger instanceof index_2.ConsoleLogger) {
                typescript_1.default.sys.clearScreen?.();
            }
            firstStatusReport = false;
            this.logger.info(typescript_1.default.flattenDiagnosticMessageText(status.messageText, newLine));
        });
        let successFinished = true;
        let currentProgram;
        const runSuccess = () => {
            if (!currentProgram) {
                return;
            }
            if (successFinished) {
                if (this.options.getValue("emit") === "both") {
                    currentProgram.emit();
                }
                this.logger.resetErrors();
                this.logger.resetWarnings();
                const entryPoints = (0, entry_point_1.getWatchEntryPoints)(this.logger, this.options, currentProgram);
                if (!entryPoints) {
                    return;
                }
                const project = this.converter.convert(entryPoints);
                currentProgram = undefined;
                successFinished = false;
                void success(project).then(() => {
                    successFinished = true;
                    runSuccess();
                });
            }
        };
        const origCreateProgram = host.createProgram;
        host.createProgram = (rootNames, options, host, oldProgram, configDiagnostics, references) => {
            // If we always do this, we'll get a crash the second time a program is created.
            if (rootNames !== undefined) {
                options = this.options.fixCompilerOptions(options || {});
            }
            return origCreateProgram(rootNames, options, host, oldProgram, configDiagnostics, references);
        };
        const origAfterProgramCreate = host.afterProgramCreate;
        host.afterProgramCreate = (program) => {
            if (typescript_1.default.getPreEmitDiagnostics(program.getProgram()).length === 0) {
                currentProgram = program.getProgram();
                runSuccess();
            }
            origAfterProgramCreate?.(program);
        };
        typescript_1.default.createWatchProgram(host);
    }
    validate(project) {
        const checks = this.options.getValue("validation");
        const start = Date.now();
        // No point in validating exports when merging. Warnings will have already been emitted when
        // creating the project jsons that this run merges together.
        if (checks.notExported &&
            this.entryPointStrategy !== entry_point_1.EntryPointStrategy.Merge) {
            (0, exports_1.validateExports)(project, this.logger, this.options.getValue("intentionallyNotExported"));
        }
        if (checks.notDocumented) {
            (0, documentation_1.validateDocumentation)(project, this.logger, this.options.getValue("requiredToBeDocumented"));
        }
        if (checks.invalidLink) {
            (0, links_1.validateLinks)(project, this.logger);
        }
        this.trigger(Application_1.EVENT_VALIDATE_PROJECT, project);
        this.logger.verbose(`Validation took ${Date.now() - start}ms`);
    }
    /**
     * Render HTML for the given project
     */
    async generateDocs(project, out) {
        const start = Date.now();
        out = Path.resolve(out);
        await this.renderer.render(project, out);
        if (this.logger.hasErrors()) {
            this.logger.error("Documentation could not be generated due to the errors above.");
        }
        else {
            this.logger.info(`Documentation generated at ${(0, paths_1.nicePath)(out)}`);
            this.logger.verbose(`HTML rendering took ${Date.now() - start}ms`);
        }
    }
    /**
     * Write the reflections to a json file.
     *
     * @param out The path and file name of the target file.
     * @returns Whether the JSON file could be written successfully.
     */
    async generateJson(project, out) {
        const start = Date.now();
        out = Path.resolve(out);
        const ser = this.serializer.projectToObject(project, process.cwd());
        const space = this.options.getValue("pretty") ? "\t" : "";
        await (0, index_2.writeFile)(out, JSON.stringify(ser, null, space));
        this.logger.info(`JSON written to ${(0, paths_1.nicePath)(out)}`);
        this.logger.verbose(`JSON rendering took ${Date.now() - start}ms`);
    }
    /**
     * Print the version number.
     */
    toString() {
        return [
            "",
            `TypeDoc ${Application_1.VERSION}`,
            `Using TypeScript ${this.getTypeScriptVersion()} from ${this.getTypeScriptPath()}`,
            "",
        ].join("\n");
    }
    _convertPackages() {
        if (!this.options.isSet("entryPoints")) {
            this.logger.error("No entry points provided to packages mode, documentation cannot be generated.");
            return;
        }
        const packageDirs = (0, entry_point_1.getPackageDirectories)(this.logger, this.options, this.options.getValue("entryPoints"));
        if (packageDirs.length === 0) {
            this.logger.error("Failed to find any packages, ensure you have provided at least one directory as an entry point containing package.json");
            return;
        }
        const origOptions = this.options;
        const projects = [];
        // Generate a json file for each package
        for (const dir of packageDirs) {
            this.logger.info(`Converting project at ${(0, paths_1.nicePath)(dir)}`);
            const opts = origOptions.copyForPackage(dir);
            // Invalid links should only be reported after everything has been merged.
            opts.setValue("validation", { invalidLink: false });
            opts.read(this.logger, dir);
            if (opts.getValue("entryPointStrategy") ===
                entry_point_1.EntryPointStrategy.Packages) {
                this.logger.error(`Project at ${(0, paths_1.nicePath)(dir)} has entryPointStrategy set to packages, but nested packages are not supported.`);
                continue;
            }
            this.options = opts;
            const project = this.convert();
            if (project) {
                this.validate(project);
                projects.push(this.serializer.projectToObject(project, process.cwd()));
            }
            (0, abstract_1.resetReflectionID)();
        }
        this.options = origOptions;
        this.logger.info(`Merging converted projects`);
        if (projects.length !== packageDirs.length) {
            this.logger.error("Failed to convert one or more packages, result will not be merged together.");
            return;
        }
        const result = this.deserializer.reviveProjects(this.options.getValue("name") || "Documentation", projects);
        this.trigger(application_events_1.ApplicationEvents.REVIVE, result);
        return result;
    }
    _merge() {
        const start = Date.now();
        if (!this.options.isSet("entryPoints")) {
            this.logger.error("No entry points provided to merge.");
            return;
        }
        const rootDir = (0, fs_1.deriveRootDir)(this.entryPoints);
        const entryPoints = this.entryPoints.flatMap((entry) => {
            const result = (0, fs_1.glob)(entry, rootDir);
            if (result.length === 0) {
                this.logger.warn(`The entrypoint glob ${(0, paths_1.nicePath)(entry)} did not match any files.`);
            }
            else {
                this.logger.verbose(`Expanded ${(0, paths_1.nicePath)(entry)} to:\n\t${result
                    .map(paths_1.nicePath)
                    .join("\n\t")}`);
            }
            return result;
        });
        const jsonProjects = entryPoints.map((path) => {
            try {
                return JSON.parse((0, fs_1.readFile)(path));
            }
            catch {
                this.logger.error(`Failed to parse file at ${(0, paths_1.nicePath)(path)} as json.`);
                return null;
            }
        });
        if (this.logger.hasErrors())
            return;
        const result = this.deserializer.reviveProjects(this.options.getValue("name"), jsonProjects);
        this.logger.verbose(`Reviving projects took ${Date.now() - start}ms`);
        this.trigger(application_events_1.ApplicationEvents.REVIVE, result);
        return result;
    }
};
/**
 * The version number of TypeDoc.
 */
Application.VERSION = packageInfo.version;
/**
 * Emitted after plugins have been loaded and options have been read, but before they have been frozen.
 * The listener will be given an instance of {@link Application}.
 */
Application.EVENT_BOOTSTRAP_END = application_events_1.ApplicationEvents.BOOTSTRAP_END;
/**
 * Emitted after a project has been deserialized from JSON.
 * The listener will be given an instance of {@link ProjectReflection}.
 */
Application.EVENT_PROJECT_REVIVE = application_events_1.ApplicationEvents.REVIVE;
/**
 * Emitted when validation is being run.
 * The listener will be given an instance of {@link ProjectReflection}.
 */
Application.EVENT_VALIDATE_PROJECT = application_events_1.ApplicationEvents.VALIDATE_PROJECT;
__decorate([
    (0, utils_1.BindOption)("skipErrorChecking")
], Application.prototype, "skipErrorChecking", void 0);
__decorate([
    (0, utils_1.BindOption)("entryPointStrategy")
], Application.prototype, "entryPointStrategy", void 0);
__decorate([
    (0, utils_1.BindOption)("entryPoints")
], Application.prototype, "entryPoints", void 0);
Application = Application_1 = __decorate([
    (0, component_1.Component)({ name: "application", internal: true })
], Application);
exports.Application = Application;
