"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.BindOption = exports.Options = void 0;
const declaration_1 = require("./declaration");
const array_1 = require("../array");
const declaration_2 = require("./declaration");
const sources_1 = require("./sources");
const help_1 = require("./help");
const optionSnapshots = new WeakMap();
/**
 * Maintains a collection of option declarations split into TypeDoc options
 * and TypeScript options. Ensures options are of the correct type for calling
 * code.
 *
 * ### Option Discovery
 *
 * Since plugins commonly add custom options, and TypeDoc does not permit options which have
 * not been declared to be set, options must be read twice. The first time options are read,
 * a noop logger is passed so that any errors are ignored. Then, after loading plugins, options
 * are read again, this time with the logger specified by the application.
 *
 * Options are read in a specific order.
 * 1. argv (0) - Must be read first since it should change the files read when
 *    passing --options or --tsconfig.
 * 2. typedoc-json (100) - Read next so that it can specify the tsconfig.json file to read.
 * 3. tsconfig-json (200) - Last config file reader, cannot specify the typedoc.json file to read.
 * 4. argv (300) - Read argv again since any options set there should override those set in config
 *    files.
 */
class Options {
    constructor(logger) {
        this._readers = [];
        this._declarations = new Map();
        this._values = {};
        this._setOptions = new Set();
        this._compilerOptions = {};
        this._fileNames = [];
        this._projectReferences = [];
        this._logger = logger;
        (0, sources_1.addTypeDocOptions)(this);
    }
    /**
     * Clones the options, intended for use in packages mode.
     */
    copyForPackage(packageDir) {
        const options = new Options(this._logger);
        options.packageDir = packageDir;
        options._readers = this._readers.filter((reader) => reader.supportsPackages);
        options._declarations = new Map(this._declarations);
        return options;
    }
    /**
     * Marks the options as readonly, enables caching when fetching options, which improves performance.
     */
    freeze() {
        Object.freeze(this._values);
    }
    /**
     * Checks if the options object has been frozen, preventing future changes to option values.
     */
    isFrozen() {
        return Object.isFrozen(this._values);
    }
    /**
     * Take a snapshot of option values now, used in tests only.
     * @internal
     */
    snapshot() {
        const key = {};
        optionSnapshots.set(key, {
            values: JSON.stringify(this._values),
            set: new Set(this._setOptions),
        });
        return key;
    }
    /**
     * Take a snapshot of option values now, used in tests only.
     * @internal
     */
    restore(snapshot) {
        const data = optionSnapshots.get(snapshot);
        this._values = JSON.parse(data.values);
        this._setOptions = new Set(data.set);
    }
    /**
     * Sets the logger used when an option declaration fails to be added.
     * @param logger
     */
    setLogger(logger) {
        this._logger = logger;
    }
    reset(name) {
        if (name != null) {
            const declaration = this.getDeclaration(name);
            if (!declaration) {
                throw new Error("Cannot reset an option which has not been declared.");
            }
            this._values[declaration.name] = (0, declaration_2.getDefaultValue)(declaration);
            this._setOptions.delete(declaration.name);
        }
        else {
            for (const declaration of this.getDeclarations()) {
                this._values[declaration.name] = (0, declaration_2.getDefaultValue)(declaration);
            }
            this._setOptions.clear();
            this._compilerOptions = {};
            this._fileNames = [];
        }
    }
    /**
     * Adds an option reader that will be used to read configuration values
     * from the command line, configuration files, or other locations.
     * @param reader
     */
    addReader(reader) {
        (0, array_1.insertOrderSorted)(this._readers, reader);
    }
    read(logger, cwd = process.cwd()) {
        for (const reader of this._readers) {
            reader.read(this, logger, cwd);
        }
    }
    addDeclaration(declaration) {
        const decl = this.getDeclaration(declaration.name);
        if (decl) {
            this._logger.error(`The option ${declaration.name} has already been registered`);
        }
        else {
            this._declarations.set(declaration.name, declaration);
        }
        this._values[declaration.name] = (0, declaration_2.getDefaultValue)(declaration);
    }
    /**
     * Gets a declaration by one of its names.
     * @param name
     */
    getDeclaration(name) {
        return this._declarations.get(name);
    }
    /**
     * Gets all declared options.
     */
    getDeclarations() {
        return (0, array_1.unique)(this._declarations.values());
    }
    isSet(name) {
        if (!this._declarations.has(name)) {
            throw new Error("Tried to check if an undefined option was set");
        }
        return this._setOptions.has(name);
    }
    /**
     * Gets all of the TypeDoc option values defined in this option container.
     */
    getRawValues() {
        return this._values;
    }
    getValue(name) {
        const declaration = this.getDeclaration(name);
        if (!declaration) {
            const nearNames = this.getSimilarOptions(name);
            throw new Error(`Unknown option '${name}', you may have meant:\n\t${nearNames.join("\n\t")}`);
        }
        return this._values[declaration.name];
    }
    setValue(name, value, configPath) {
        if (this.isFrozen()) {
            throw new Error("Tried to modify an option value after options have been frozen.");
        }
        const declaration = this.getDeclaration(name);
        if (!declaration) {
            const nearNames = this.getSimilarOptions(name);
            throw new Error(`Tried to set an option (${name}) that was not declared. You may have meant:\n\t${nearNames.join("\n\t")}`);
        }
        let oldValue = this._values[declaration.name];
        if (typeof oldValue === "undefined")
            oldValue = (0, declaration_2.getDefaultValue)(declaration);
        const converted = (0, declaration_2.convert)(value, declaration, configPath ?? process.cwd(), oldValue);
        if (declaration.type === declaration_1.ParameterType.Flags) {
            Object.assign(this._values[declaration.name], converted);
        }
        else {
            this._values[declaration.name] = converted;
        }
        this._setOptions.add(name);
    }
    /**
     * Gets the set compiler options.
     */
    getCompilerOptions() {
        return this.fixCompilerOptions(this._compilerOptions);
    }
    /** @internal */
    fixCompilerOptions(options) {
        const overrides = this.getValue("compilerOptions");
        const result = { ...options };
        if (overrides) {
            Object.assign(result, overrides);
        }
        if (this.getValue("emit") !== "both") {
            result.noEmit = true;
            delete result.emitDeclarationOnly;
        }
        return result;
    }
    /**
     * Gets the file names discovered through reading a tsconfig file.
     */
    getFileNames() {
        return this._fileNames;
    }
    /**
     * Gets the project references - used in solution style tsconfig setups.
     */
    getProjectReferences() {
        return this._projectReferences;
    }
    /**
     * Sets the compiler options that will be used to get a TS program.
     */
    setCompilerOptions(fileNames, options, projectReferences) {
        if (this.isFrozen()) {
            throw new Error("Tried to modify an option value after options have been sealed.");
        }
        // We do this here instead of in the tsconfig reader so that API consumers which
        // supply a program to `Converter.convert` instead of letting TypeDoc create one
        // can just set the compiler options, and not need to know about this mapping.
        // It feels a bit like a hack... but it's better to have it here than to put it
        // in Application or Converter.
        if (options.stripInternal && !this.isSet("excludeInternal")) {
            this.setValue("excludeInternal", true);
        }
        this._fileNames = fileNames;
        this._compilerOptions = { ...options };
        this._projectReferences = projectReferences ?? [];
    }
    /**
     * Discover similar option names to the given name, for use in error reporting.
     */
    getSimilarOptions(missingName) {
        const results = {};
        let lowest = Infinity;
        for (const name of this._declarations.keys()) {
            const distance = editDistance(missingName, name);
            lowest = Math.min(lowest, distance);
            results[distance] || (results[distance] = []);
            results[distance].push(name);
        }
        // Experimenting a bit, it seems an edit distance of 3 is roughly the
        // right metric for relevant "similar" results without showing obviously wrong suggestions
        return results[lowest].concat(results[lowest + 1] || [], results[lowest + 2] || []);
    }
    /**
     * Get the help message to be displayed to the user if `--help` is passed.
     */
    getHelp() {
        return (0, help_1.getOptionsHelp)(this);
    }
}
exports.Options = Options;
function BindOption(name) {
    return function (target, key) {
        Object.defineProperty(target, key, {
            get() {
                const options = "options" in this ? this.options : this.application.options;
                const value = options.getValue(name);
                return value;
            },
            enumerable: true,
            configurable: true,
        });
    };
}
exports.BindOption = BindOption;
// Based on https://en.wikipedia.org/wiki/Levenshtein_distance#Iterative_with_two_matrix_rows
// Slightly modified for improved match results for options
function editDistance(s, t) {
    if (s.length < t.length)
        return editDistance(t, s);
    let v0 = Array.from({ length: t.length + 1 }, (_, i) => i);
    let v1 = Array.from({ length: t.length + 1 }, () => 0);
    for (let i = 0; i < s.length; i++) {
        v1[0] = i + 1;
        for (let j = 0; j < s.length; j++) {
            const deletionCost = v0[j + 1] + 1;
            const insertionCost = v1[j] + 1;
            let substitutionCost;
            if (s[i] === t[j]) {
                substitutionCost = v0[j];
            }
            else if (s[i]?.toUpperCase() === t[j]?.toUpperCase()) {
                substitutionCost = v0[j] + 1;
            }
            else {
                substitutionCost = v0[j] + 3;
            }
            v1[j + 1] = Math.min(deletionCost, insertionCost, substitutionCost);
        }
        [v0, v1] = [v1, v0];
    }
    return v0[t.length];
}
