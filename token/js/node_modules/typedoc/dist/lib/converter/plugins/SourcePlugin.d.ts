import { ConverterComponent } from "../components";
/**
 * A handler that attaches source file information to reflections.
 */
export declare class SourcePlugin extends ConverterComponent {
    readonly disableSources: boolean;
    readonly gitRevision: string;
    readonly gitRemote: string;
    readonly sourceLinkTemplate: string;
    readonly basePath: string;
    /**
     * All file names to find the base path from.
     */
    private fileNames;
    /**
     * List of known repositories.
     */
    private repositories;
    /**
     * List of paths known to be not under git control.
     */
    private ignoredPaths;
    /**
     * Create a new SourceHandler instance.
     */
    initialize(): void;
    private onEnd;
    /**
     * Triggered when the converter has created a declaration reflection.
     *
     * Attach the current source file to the {@link DeclarationReflection.sources} array.
     *
     * @param _context  The context object describing the current state the converter is in.
     * @param reflection  The reflection that is currently processed.
     */
    private onDeclaration;
    private onSignature;
    /**
     * Triggered when the converter begins resolving a project.
     *
     * @param context  The context object describing the current state the converter is in.
     */
    private onBeginResolve;
    /**
     * Check whether the given file is placed inside a repository.
     *
     * @param fileName  The name of the file a repository should be looked for.
     * @returns The found repository info or undefined.
     */
    private getRepository;
}
