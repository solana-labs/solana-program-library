import type * as ts from "typescript";
import type { NeverIfInternal } from "..";
import type { Application } from "../../..";
import type { Logger } from "../loggers";
import { DeclarationOption, KeyToDeclaration, TypeDocOptionMap, TypeDocOptions, TypeDocOptionValues } from "./declaration";
/**
 * Describes an option reader that discovers user configuration and converts it to the
 * TypeDoc format.
 */
export interface OptionsReader {
    /**
     * Readers will be processed according to their orders.
     * A higher order indicates that the reader should be called *later*.
     *
     * Note that to preserve expected behavior, the argv reader must have both the lowest
     * order so that it may set the location of config files used by other readers and
     * the highest order so that it can override settings from lower order readers.
     */
    readonly order: number;
    /**
     * The name of this reader so that it may be removed by plugins without the plugin
     * accessing the instance performing the read. Multiple readers may have the same
     * name.
     */
    readonly name: string;
    /**
     * Flag to indicate that this reader should be included in sub-options objects created
     * to read options for packages mode.
     */
    readonly supportsPackages: boolean;
    /**
     * Read options from the reader's source and place them in the options parameter.
     * Options without a declared name may be treated as if they were declared with type
     * {@link ParameterType.Mixed}. Options which have been declared must be converted to the
     * correct type. As an alternative to doing this conversion in the reader,
     * the reader may use {@link Options.setValue}, which will correctly convert values.
     * @param container the options container that provides declarations
     * @param logger logger to be used to report errors
     * @param cwd the directory which should be treated as the current working directory for option file discovery
     */
    read(container: Options, logger: Logger, cwd: string): void;
}
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
export declare class Options {
    private _readers;
    private _declarations;
    private _values;
    private _setOptions;
    private _compilerOptions;
    private _fileNames;
    private _projectReferences;
    private _logger;
    /**
     * In packages mode, the directory of the package being converted.
     */
    packageDir?: string;
    constructor(logger: Logger);
    /**
     * Clones the options, intended for use in packages mode.
     */
    copyForPackage(packageDir: string): Options;
    /**
     * Marks the options as readonly, enables caching when fetching options, which improves performance.
     */
    freeze(): void;
    /**
     * Checks if the options object has been frozen, preventing future changes to option values.
     */
    isFrozen(): boolean;
    /**
     * Take a snapshot of option values now, used in tests only.
     * @internal
     */
    snapshot(): {
        __optionSnapshot: never;
    };
    /**
     * Take a snapshot of option values now, used in tests only.
     * @internal
     */
    restore(snapshot: {
        __optionSnapshot: never;
    }): void;
    /**
     * Sets the logger used when an option declaration fails to be added.
     * @param logger
     */
    setLogger(logger: Logger): void;
    /**
     * Resets the option bag to all default values.
     * If a name is provided, will only reset that name.
     */
    reset(name?: keyof TypeDocOptions): void;
    reset(name?: NeverIfInternal<string>): void;
    /**
     * Adds an option reader that will be used to read configuration values
     * from the command line, configuration files, or other locations.
     * @param reader
     */
    addReader(reader: OptionsReader): void;
    read(logger: Logger, cwd?: string): void;
    /**
     * Adds an option declaration to the container with extra type checking to ensure that
     * the runtime type is consistent with the declared type.
     * @param declaration The option declaration that should be added.
     */
    addDeclaration<K extends keyof TypeDocOptions>(declaration: {
        name: K;
    } & KeyToDeclaration<K>): void;
    /**
     * Adds an option declaration to the container.
     * @param declaration The option declaration that should be added.
     */
    addDeclaration(declaration: NeverIfInternal<Readonly<DeclarationOption>>): void;
    /**
     * Gets a declaration by one of its names.
     * @param name
     */
    getDeclaration(name: string): Readonly<DeclarationOption> | undefined;
    /**
     * Gets all declared options.
     */
    getDeclarations(): Readonly<DeclarationOption>[];
    /**
     * Checks if the given option's value is deeply strict equal to the default.
     * @param name
     */
    isSet(name: keyof TypeDocOptions): boolean;
    isSet(name: NeverIfInternal<string>): boolean;
    /**
     * Gets all of the TypeDoc option values defined in this option container.
     */
    getRawValues(): Readonly<Partial<TypeDocOptions>>;
    /**
     * Gets a value for the given option key, throwing if the option has not been declared.
     * @param name
     */
    getValue<K extends keyof TypeDocOptions>(name: K): TypeDocOptionValues[K];
    getValue(name: NeverIfInternal<string>): unknown;
    /**
     * Sets the given declared option. Throws if setting the option fails.
     * @param name
     * @param value
     * @param configPath the directory to resolve Path type values against
     */
    setValue<K extends keyof TypeDocOptions>(name: K, value: TypeDocOptions[K], configPath?: string): void;
    setValue(name: NeverIfInternal<string>, value: NeverIfInternal<unknown>, configPath?: NeverIfInternal<string>): void;
    /**
     * Gets the set compiler options.
     */
    getCompilerOptions(): ts.CompilerOptions;
    /** @internal */
    fixCompilerOptions(options: Readonly<ts.CompilerOptions>): ts.CompilerOptions;
    /**
     * Gets the file names discovered through reading a tsconfig file.
     */
    getFileNames(): readonly string[];
    /**
     * Gets the project references - used in solution style tsconfig setups.
     */
    getProjectReferences(): readonly ts.ProjectReference[];
    /**
     * Sets the compiler options that will be used to get a TS program.
     */
    setCompilerOptions(fileNames: readonly string[], options: ts.CompilerOptions, projectReferences: readonly ts.ProjectReference[] | undefined): void;
    /**
     * Discover similar option names to the given name, for use in error reporting.
     */
    getSimilarOptions(missingName: string): string[];
    /**
     * Get the help message to be displayed to the user if `--help` is passed.
     */
    getHelp(): string;
}
/**
 * Binds an option to the given property. Does not register the option.
 *
 * Note: This is a legacy experimental decorator, and will not work with TS 5.0 decorators
 *
 * @since v0.16.3
 */
export declare function BindOption<K extends keyof TypeDocOptionMap>(name: K): <IK extends PropertyKey>(target: ({
    application: Application;
} | {
    options: Options;
}) & {
    [K2 in IK]: TypeDocOptionValues[K];
}, key: IK) => void;
/**
 * Binds an option to the given property. Does not register the option.
 *
 * Note: This is a legacy experimental decorator, and will not work with TS 5.0 decorators
 *
 * @since v0.16.3
 *
 * @privateRemarks
 * This overload is intended for plugin use only with looser type checks. Do not use internally.
 */
export declare function BindOption(name: NeverIfInternal<string>): (target: {
    application: Application;
} | {
    options: Options;
}, key: PropertyKey) => void;
