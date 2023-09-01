import ts from "typescript";
import type { Application } from "../application";
import { Comment, CommentDisplayPart, ProjectReflection, Reflection, ReflectionSymbolId, SomeType } from "../models/index";
import { Context } from "./context";
import { ConverterComponent } from "./components";
import { ChildableComponent } from "../utils/component";
import { MinimalSourceFile } from "../utils";
import type { DocumentationEntryPoint } from "../utils/entry-point";
import type { CommentParserConfig } from "./comments";
import type { CommentStyle, ValidationOptions } from "../utils/options/declaration";
import { ExternalSymbolResolver, ExternalResolveResult } from "./comments/linkResolver";
import type { DeclarationReference } from "./comments/declarationReference";
/**
 * Compiles source files using TypeScript and converts compiler symbols to reflections.
 */
export declare class Converter extends ChildableComponent<Application, ConverterComponent> {
    /** @internal */
    externalPattern: string[];
    private externalPatternCache?;
    private excludeCache?;
    /** @internal */
    excludeExternals: boolean;
    /** @internal */
    excludeNotDocumented: boolean;
    /** @internal */
    excludePrivate: boolean;
    /** @internal */
    excludeProtected: boolean;
    /** @internal */
    excludeReferences: boolean;
    /** @internal */
    commentStyle: CommentStyle;
    /** @internal */
    validation: ValidationOptions;
    /** @internal */
    externalSymbolLinkMappings: Record<string, Record<string, string>>;
    /** @internal */
    useTsLinkResolution: boolean;
    private _config?;
    private _externalSymbolResolvers;
    get config(): CommentParserConfig;
    /**
     * General events
     */
    /**
     * Triggered when the converter begins converting a project.
     * The listener will be given a {@link Context} object.
     * @event
     */
    static readonly EVENT_BEGIN: "begin";
    /**
     * Triggered when the converter has finished converting a project.
     * The listener will be given a {@link Context} object.
     * @event
     */
    static readonly EVENT_END: "end";
    /**
     * Factory events
     */
    /**
     * Triggered when the converter has created a declaration reflection.
     * The listener will be given {@link Context} and a {@link Models.DeclarationReflection}.
     * @event
     */
    static readonly EVENT_CREATE_DECLARATION: "createDeclaration";
    /**
     * Triggered when the converter has created a signature reflection.
     * The listener will be given {@link Context}, {@link Models.SignatureReflection} | {@link Models.ProjectReflection} the declaration,
     * `ts.SignatureDeclaration | ts.IndexSignatureDeclaration | ts.JSDocSignature | undefined`,
     * and `ts.Signature | undefined`. The signature will be undefined if the created signature is an index signature.
     * @event
     */
    static readonly EVENT_CREATE_SIGNATURE: "createSignature";
    /**
     * Triggered when the converter has created a parameter reflection.
     * The listener will be given {@link Context}, {@link Models.ParameterReflection} and a `ts.Node?`
     * @event
     */
    static readonly EVENT_CREATE_PARAMETER: "createParameter";
    /**
     * Triggered when the converter has created a type parameter reflection.
     * The listener will be given {@link Context} and a {@link Models.TypeParameterReflection}
     * @event
     */
    static readonly EVENT_CREATE_TYPE_PARAMETER: "createTypeParameter";
    /**
     * Resolve events
     */
    /**
     * Triggered when the converter begins resolving a project.
     * The listener will be given {@link Context}.
     * @event
     */
    static readonly EVENT_RESOLVE_BEGIN: "resolveBegin";
    /**
     * Triggered when the converter resolves a reflection.
     * The listener will be given {@link Context} and a {@link Reflection}.
     * @event
     */
    static readonly EVENT_RESOLVE: "resolveReflection";
    /**
     * Triggered when the converter has finished resolving a project.
     * The listener will be given {@link Context}.
     * @event
     */
    static readonly EVENT_RESOLVE_END: "resolveEnd";
    constructor(owner: Application);
    /**
     * Compile the given source files and create a project reflection for them.
     */
    convert(entryPoints: readonly DocumentationEntryPoint[]): ProjectReflection;
    /** @internal */
    convertSymbol(context: Context, symbol: ts.Symbol, exportSymbol?: ts.Symbol): void;
    /**
     * Convert the given TypeScript type into its TypeDoc type reflection.
     *
     * @param context  The context object describing the current state the converter is in.
     * @param referenceTarget The target to be used to attempt to resolve reference types
     * @returns The TypeDoc type reflection representing the given node and type.
     * @internal
     */
    convertType(context: Context, node: ts.TypeNode | ts.Type | undefined): SomeType;
    /**
     * Parse the given file into a comment. Intended to be used with markdown files.
     */
    parseRawComment(file: MinimalSourceFile): Comment;
    /**
     * Adds a new resolver that the theme can use to try to figure out how to link to a symbol declared
     * by a third-party library which is not included in the documentation.
     *
     * The resolver function will be passed a declaration reference which it can attempt to resolve. If
     * resolution fails, the function should return undefined.
     *
     * Note: This will be used for both references to types declared in node_modules (in which case the
     * reference passed will have the `moduleSource` set and the `symbolReference` will navigate via `.`)
     * and user defined \{\@link\} tags which cannot be resolved. If the link being resolved is inferred
     * from a type, then no `part` will be passed to the resolver function.
     * @since 0.22.14
     */
    addUnknownSymbolResolver(resolver: ExternalSymbolResolver): void;
    /** @internal */
    resolveExternalLink(ref: DeclarationReference, refl: Reflection, part: CommentDisplayPart | undefined, symbolId: ReflectionSymbolId | undefined): ExternalResolveResult | string | undefined;
    resolveLinks(comment: Comment, owner: Reflection): void;
    resolveLinks(parts: readonly CommentDisplayPart[], owner: Reflection): CommentDisplayPart[];
    /**
     * Compile the files within the given context and convert the compiler symbols to reflections.
     *
     * @param context  The context object describing the current state the converter is in.
     * @returns An array containing all errors generated by the TypeScript compiler.
     */
    private compile;
    private convertExports;
    private convertReExports;
    /**
     * Resolve the project within the given context.
     *
     * @param context  The context object describing the current state the converter is in.
     * @returns The final project reflection.
     */
    private resolve;
    /**
     * Used to determine if we should immediately bail when creating a reflection.
     * Note: This should not be used for excludeNotDocumented because we don't have enough
     * information at this point since comment discovery hasn't happened.
     * @internal
     */
    shouldIgnore(symbol: ts.Symbol): boolean;
    private isExcluded;
    /** @internal */
    isExternal(symbol: ts.Symbol): boolean;
    private _buildCommentParserConfig;
}
