import { Converter } from "./converter/index";
import { Renderer } from "./output/renderer";
import { Deserializer, Serializer } from "./serialization";
import type { ProjectReflection } from "./models/index";
import { Logger } from "./utils/index";
import { AbstractComponent, ChildableComponent } from "./utils/component";
import { Options } from "./utils";
import type { TypeDocOptions } from "./utils/options/declaration";
import { DocumentationEntryPoint, EntryPointStrategy } from "./utils/entry-point";
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
export declare class Application extends ChildableComponent<Application, AbstractComponent<Application>> {
    /**
     * The converter used to create the declaration reflections.
     */
    converter: Converter;
    /**
     * The renderer used to generate the documentation output.
     */
    renderer: Renderer;
    /**
     * The serializer used to generate JSON output.
     */
    serializer: Serializer;
    /**
     * The deserializer used to restore previously serialized JSON output.
     */
    deserializer: Deserializer;
    /**
     * The logger that should be used to output messages.
     */
    logger: Logger;
    options: Options;
    /** @internal */
    readonly skipErrorChecking: boolean;
    /** @internal */
    readonly entryPointStrategy: EntryPointStrategy;
    /** @internal */
    readonly entryPoints: string[];
    /**
     * The version number of TypeDoc.
     */
    static VERSION: string;
    /**
     * Emitted after plugins have been loaded and options have been read, but before they have been frozen.
     * The listener will be given an instance of {@link Application}.
     */
    static readonly EVENT_BOOTSTRAP_END: string;
    /**
     * Emitted after a project has been deserialized from JSON.
     * The listener will be given an instance of {@link ProjectReflection}.
     */
    static readonly EVENT_PROJECT_REVIVE: string;
    /**
     * Emitted when validation is being run.
     * The listener will be given an instance of {@link ProjectReflection}.
     */
    static readonly EVENT_VALIDATE_PROJECT: string;
    /**
     * Create a new TypeDoc application instance.
     */
    constructor();
    /**
     * Initialize TypeDoc, loading plugins if applicable.
     */
    bootstrapWithPlugins(options?: Partial<TypeDocOptions>): Promise<void>;
    /**
     * Initialize TypeDoc without loading plugins.
     */
    bootstrap(options?: Partial<TypeDocOptions>): void;
    private setOptions;
    /**
     * Return the path to the TypeScript compiler.
     */
    getTypeScriptPath(): string;
    getTypeScriptVersion(): string;
    /**
     * Gets the entry points to be documented according to the current `entryPoints` and `entryPointStrategy` options.
     * May return undefined if entry points fail to be expanded.
     */
    getEntryPoints(): DocumentationEntryPoint[] | undefined;
    /**
     * Run the converter for the given set of files and return the generated reflections.
     *
     * @returns An instance of ProjectReflection on success, undefined otherwise.
     */
    convert(): ProjectReflection | undefined;
    convertAndWatch(success: (project: ProjectReflection) => Promise<void>): void;
    validate(project: ProjectReflection): void;
    /**
     * Render HTML for the given project
     */
    generateDocs(project: ProjectReflection, out: string): Promise<void>;
    /**
     * Write the reflections to a json file.
     *
     * @param out The path and file name of the target file.
     * @returns Whether the JSON file could be written successfully.
     */
    generateJson(project: ProjectReflection, out: string): Promise<void>;
    /**
     * Print the version number.
     */
    toString(): string;
    private _convertPackages;
    private _merge;
}
