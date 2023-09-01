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
var Converter_1;
Object.defineProperty(exports, "__esModule", { value: true });
exports.Converter = void 0;
const typescript_1 = __importDefault(require("typescript"));
const index_1 = require("../models/index");
const context_1 = require("./context");
const components_1 = require("./components");
const component_1 = require("../utils/component");
const utils_1 = require("../utils");
const types_1 = require("./types");
const converter_events_1 = require("./converter-events");
const symbols_1 = require("./symbols");
const paths_1 = require("../utils/paths");
const enum_1 = require("../utils/enum");
const parser_1 = require("./comments/parser");
const rawLexer_1 = require("./comments/rawLexer");
const linkResolver_1 = require("./comments/linkResolver");
/**
 * Compiles source files using TypeScript and converts compiler symbols to reflections.
 */
let Converter = Converter_1 = class Converter extends component_1.ChildableComponent {
    get config() {
        return this._config || this._buildCommentParserConfig();
    }
    constructor(owner) {
        super(owner);
        this._externalSymbolResolvers = [];
        this.addUnknownSymbolResolver((ref) => {
            // Require global links, matching local ones will likely hide mistakes where the
            // user meant to link to a local type.
            if (ref.resolutionStart !== "global" || !ref.symbolReference) {
                return;
            }
            const modLinks = this.externalSymbolLinkMappings[ref.moduleSource ?? "global"];
            if (typeof modLinks !== "object") {
                return;
            }
            let name = "";
            if (ref.symbolReference.path) {
                name += ref.symbolReference.path.map((p) => p.path).join(".");
            }
            if (ref.symbolReference.meaning) {
                name += ":" + ref.symbolReference.meaning;
            }
            if (typeof modLinks[name] === "string") {
                return modLinks[name];
            }
            if (typeof modLinks["*"] === "string") {
                return modLinks["*"];
            }
        });
    }
    /**
     * Compile the given source files and create a project reflection for them.
     */
    convert(entryPoints) {
        const programs = entryPoints.map((e) => e.program);
        this.externalPatternCache = void 0;
        const project = new index_1.ProjectReflection(this.application.options.getValue("name"));
        const context = new context_1.Context(this, programs, project);
        this.trigger(Converter_1.EVENT_BEGIN, context);
        this.compile(entryPoints, context);
        this.resolve(context);
        this.trigger(Converter_1.EVENT_END, context);
        this._config = undefined;
        return project;
    }
    /** @internal */
    convertSymbol(context, symbol, exportSymbol) {
        (0, symbols_1.convertSymbol)(context, symbol, exportSymbol);
    }
    /**
     * Convert the given TypeScript type into its TypeDoc type reflection.
     *
     * @param context  The context object describing the current state the converter is in.
     * @param referenceTarget The target to be used to attempt to resolve reference types
     * @returns The TypeDoc type reflection representing the given node and type.
     * @internal
     */
    convertType(context, node) {
        return (0, types_1.convertType)(context, node);
    }
    /**
     * Parse the given file into a comment. Intended to be used with markdown files.
     */
    parseRawComment(file) {
        return (0, parser_1.parseComment)((0, rawLexer_1.lexCommentString)(file.text), this.config, file, this.application.logger);
    }
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
    addUnknownSymbolResolver(resolver) {
        this._externalSymbolResolvers.push(resolver);
    }
    /** @internal */
    resolveExternalLink(ref, refl, part, symbolId) {
        for (const resolver of this._externalSymbolResolvers) {
            const resolved = resolver(ref, refl, part, symbolId);
            if (resolved)
                return resolved;
        }
    }
    resolveLinks(comment, owner) {
        if (comment instanceof index_1.Comment) {
            (0, linkResolver_1.resolveLinks)(comment, owner, (ref, part, refl, id) => this.resolveExternalLink(ref, part, refl, id));
        }
        else {
            return (0, linkResolver_1.resolvePartLinks)(owner, comment, (ref, part, refl, id) => this.resolveExternalLink(ref, part, refl, id));
        }
    }
    /**
     * Compile the files within the given context and convert the compiler symbols to reflections.
     *
     * @param context  The context object describing the current state the converter is in.
     * @returns An array containing all errors generated by the TypeScript compiler.
     */
    compile(entryPoints, context) {
        const entries = entryPoints.map((e) => {
            return {
                entryPoint: e,
                context: undefined,
            };
        });
        entries.forEach((e) => {
            context.setActiveProgram(e.entryPoint.program);
            e.context = this.convertExports(context, e.entryPoint, entries.length === 1);
        });
        for (const { entryPoint, context } of entries) {
            // active program is already set on context
            // if we don't have a context, then this entry point is being ignored
            if (context) {
                this.convertReExports(context, entryPoint.sourceFile);
            }
        }
        context.setActiveProgram(undefined);
    }
    convertExports(context, entryPoint, singleEntryPoint) {
        const node = entryPoint.sourceFile;
        const entryName = entryPoint.displayName;
        const symbol = getSymbolForModuleLike(context, node);
        let moduleContext;
        if (singleEntryPoint) {
            // Special case for when we're giving a single entry point, we don't need to
            // create modules for each entry. Register the project as this module.
            context.project.registerReflection(context.project, symbol);
            context.project.comment = symbol
                ? context.getComment(symbol, context.project.kind)
                : context.getFileComment(node);
            context.trigger(Converter_1.EVENT_CREATE_DECLARATION, context.project);
            moduleContext = context;
        }
        else {
            const reflection = context.createDeclarationReflection(index_1.ReflectionKind.Module, symbol, void 0, entryName);
            if (!reflection.comment && !symbol) {
                reflection.comment = context.getFileComment(node);
            }
            if (entryPoint.readmeFile) {
                const readme = (0, utils_1.readFile)(entryPoint.readmeFile);
                const comment = this.parseRawComment(new utils_1.MinimalSourceFile(readme, entryPoint.readmeFile));
                if (comment.blockTags.length || comment.modifierTags.size) {
                    const ignored = [
                        ...comment.blockTags.map((tag) => tag.tag),
                        ...comment.modifierTags,
                    ];
                    context.logger.warn(`Block and modifier tags will be ignored within the readme:\n\t${ignored.join("\n\t")}`);
                }
                reflection.readme = comment.summary;
            }
            reflection.packageVersion = entryPoint.version;
            context.finalizeDeclarationReflection(reflection);
            moduleContext = context.withScope(reflection);
        }
        const allExports = getExports(context, node, symbol);
        for (const exp of allExports.filter((exp) => isDirectExport(context.resolveAliasedSymbol(exp), node))) {
            (0, symbols_1.convertSymbol)(moduleContext, exp);
        }
        return moduleContext;
    }
    convertReExports(moduleContext, node) {
        for (const exp of getExports(moduleContext, node, moduleContext.project.getSymbolFromReflection(moduleContext.scope)).filter((exp) => !isDirectExport(moduleContext.resolveAliasedSymbol(exp), node))) {
            (0, symbols_1.convertSymbol)(moduleContext, exp);
        }
    }
    /**
     * Resolve the project within the given context.
     *
     * @param context  The context object describing the current state the converter is in.
     * @returns The final project reflection.
     */
    resolve(context) {
        this.trigger(Converter_1.EVENT_RESOLVE_BEGIN, context);
        const project = context.project;
        for (const reflection of Object.values(project.reflections)) {
            this.trigger(Converter_1.EVENT_RESOLVE, context, reflection);
        }
        this.trigger(Converter_1.EVENT_RESOLVE_END, context);
    }
    /**
     * Used to determine if we should immediately bail when creating a reflection.
     * Note: This should not be used for excludeNotDocumented because we don't have enough
     * information at this point since comment discovery hasn't happened.
     * @internal
     */
    shouldIgnore(symbol) {
        if (this.isExcluded(symbol)) {
            return true;
        }
        return this.excludeExternals && this.isExternal(symbol);
    }
    isExcluded(symbol) {
        this.excludeCache ?? (this.excludeCache = (0, paths_1.createMinimatch)(this.application.options.getValue("exclude")));
        const cache = this.excludeCache;
        return (symbol.getDeclarations() ?? []).some((node) => (0, paths_1.matchesAny)(cache, node.getSourceFile().fileName));
    }
    /** @internal */
    isExternal(symbol) {
        this.externalPatternCache ?? (this.externalPatternCache = (0, paths_1.createMinimatch)(this.externalPattern));
        const cache = this.externalPatternCache;
        return (symbol.getDeclarations() ?? []).some((node) => (0, paths_1.matchesAny)(cache, node.getSourceFile().fileName));
    }
    _buildCommentParserConfig() {
        this._config = {
            blockTags: new Set(this.application.options.getValue("blockTags")),
            inlineTags: new Set(this.application.options.getValue("inlineTags")),
            modifierTags: new Set(this.application.options.getValue("modifierTags")),
            jsDocCompatibility: this.application.options.getValue("jsDocCompatibility"),
        };
        return this._config;
    }
};
/**
 * General events
 */
/**
 * Triggered when the converter begins converting a project.
 * The listener will be given a {@link Context} object.
 * @event
 */
Converter.EVENT_BEGIN = converter_events_1.ConverterEvents.BEGIN;
/**
 * Triggered when the converter has finished converting a project.
 * The listener will be given a {@link Context} object.
 * @event
 */
Converter.EVENT_END = converter_events_1.ConverterEvents.END;
/**
 * Factory events
 */
/**
 * Triggered when the converter has created a declaration reflection.
 * The listener will be given {@link Context} and a {@link Models.DeclarationReflection}.
 * @event
 */
Converter.EVENT_CREATE_DECLARATION = converter_events_1.ConverterEvents.CREATE_DECLARATION;
/**
 * Triggered when the converter has created a signature reflection.
 * The listener will be given {@link Context}, {@link Models.SignatureReflection} | {@link Models.ProjectReflection} the declaration,
 * `ts.SignatureDeclaration | ts.IndexSignatureDeclaration | ts.JSDocSignature | undefined`,
 * and `ts.Signature | undefined`. The signature will be undefined if the created signature is an index signature.
 * @event
 */
Converter.EVENT_CREATE_SIGNATURE = converter_events_1.ConverterEvents.CREATE_SIGNATURE;
/**
 * Triggered when the converter has created a parameter reflection.
 * The listener will be given {@link Context}, {@link Models.ParameterReflection} and a `ts.Node?`
 * @event
 */
Converter.EVENT_CREATE_PARAMETER = converter_events_1.ConverterEvents.CREATE_PARAMETER;
/**
 * Triggered when the converter has created a type parameter reflection.
 * The listener will be given {@link Context} and a {@link Models.TypeParameterReflection}
 * @event
 */
Converter.EVENT_CREATE_TYPE_PARAMETER = converter_events_1.ConverterEvents.CREATE_TYPE_PARAMETER;
/**
 * Resolve events
 */
/**
 * Triggered when the converter begins resolving a project.
 * The listener will be given {@link Context}.
 * @event
 */
Converter.EVENT_RESOLVE_BEGIN = converter_events_1.ConverterEvents.RESOLVE_BEGIN;
/**
 * Triggered when the converter resolves a reflection.
 * The listener will be given {@link Context} and a {@link Reflection}.
 * @event
 */
Converter.EVENT_RESOLVE = converter_events_1.ConverterEvents.RESOLVE;
/**
 * Triggered when the converter has finished resolving a project.
 * The listener will be given {@link Context}.
 * @event
 */
Converter.EVENT_RESOLVE_END = converter_events_1.ConverterEvents.RESOLVE_END;
__decorate([
    (0, utils_1.BindOption)("externalPattern")
], Converter.prototype, "externalPattern", void 0);
__decorate([
    (0, utils_1.BindOption)("excludeExternals")
], Converter.prototype, "excludeExternals", void 0);
__decorate([
    (0, utils_1.BindOption)("excludeNotDocumented")
], Converter.prototype, "excludeNotDocumented", void 0);
__decorate([
    (0, utils_1.BindOption)("excludePrivate")
], Converter.prototype, "excludePrivate", void 0);
__decorate([
    (0, utils_1.BindOption)("excludeProtected")
], Converter.prototype, "excludeProtected", void 0);
__decorate([
    (0, utils_1.BindOption)("excludeReferences")
], Converter.prototype, "excludeReferences", void 0);
__decorate([
    (0, utils_1.BindOption)("commentStyle")
], Converter.prototype, "commentStyle", void 0);
__decorate([
    (0, utils_1.BindOption)("validation")
], Converter.prototype, "validation", void 0);
__decorate([
    (0, utils_1.BindOption)("externalSymbolLinkMappings")
], Converter.prototype, "externalSymbolLinkMappings", void 0);
__decorate([
    (0, utils_1.BindOption)("useTsLinkResolution")
], Converter.prototype, "useTsLinkResolution", void 0);
Converter = Converter_1 = __decorate([
    (0, component_1.Component)({
        name: "converter",
        internal: true,
        childClass: components_1.ConverterComponent,
    })
], Converter);
exports.Converter = Converter;
function getSymbolForModuleLike(context, node) {
    const symbol = context.checker.getSymbolAtLocation(node) ?? node.symbol;
    if (symbol) {
        return symbol;
    }
    // This is a global file, get all symbols declared in this file...
    // this isn't the best solution, it would be nice to have all globals given to a special
    // "globals" file, but this is uncommon enough that I'm skipping it for now.
    const sourceFile = node.getSourceFile();
    const globalSymbols = context.checker
        .getSymbolsInScope(node, typescript_1.default.SymbolFlags.ModuleMember)
        .filter((s) => s.getDeclarations()?.some((d) => d.getSourceFile() === sourceFile));
    // Detect declaration files with declare module "foo" as their only export
    // and lift that up one level as the source file symbol
    if (globalSymbols.length === 1 &&
        globalSymbols[0]
            .getDeclarations()
            ?.every((declaration) => typescript_1.default.isModuleDeclaration(declaration) &&
            typescript_1.default.isStringLiteral(declaration.name))) {
        return globalSymbols[0];
    }
}
function getExports(context, node, symbol) {
    let result;
    // The generated docs aren't great, but you really ought not be using
    // this in the first place... so it's better than nothing.
    const exportEq = symbol?.exports?.get("export=");
    if (exportEq) {
        // JS users might also have exported types here.
        // We need to filter for types because otherwise static methods can show up as both
        // members of the export= class and as functions if a class is directly exported.
        result = [exportEq].concat(context.checker
            .getExportsOfModule(symbol)
            .filter((s) => !(0, enum_1.hasAnyFlag)(s.flags, typescript_1.default.SymbolFlags.Prototype | typescript_1.default.SymbolFlags.Value)));
    }
    else if (symbol) {
        result = context.checker
            .getExportsOfModule(symbol)
            .filter((s) => !(0, enum_1.hasAllFlags)(s.flags, typescript_1.default.SymbolFlags.Prototype));
        if (result.length === 0) {
            const globalDecl = node.statements.find((s) => typescript_1.default.isModuleDeclaration(s) &&
                s.flags & typescript_1.default.NodeFlags.GlobalAugmentation);
            if (globalDecl) {
                const globalSymbol = context.getSymbolAtLocation(globalDecl);
                if (globalSymbol) {
                    result = context.checker
                        .getExportsOfModule(globalSymbol)
                        .filter((exp) => exp.declarations?.some((d) => d.getSourceFile() === node));
                }
            }
        }
    }
    else {
        // Global file with no inferred top level symbol, get all symbols declared in this file.
        const sourceFile = node.getSourceFile();
        result = context.checker
            .getSymbolsInScope(node, typescript_1.default.SymbolFlags.ModuleMember)
            .filter((s) => s
            .getDeclarations()
            ?.some((d) => d.getSourceFile() === sourceFile));
    }
    // Put symbols named "default" last, #1795
    result.sort((a, b) => {
        if (a.name === "default") {
            return 1;
        }
        else if (b.name === "default") {
            return -1;
        }
        return 0;
    });
    return result;
}
function isDirectExport(symbol, file) {
    return (symbol
        .getDeclarations()
        ?.every((decl) => decl.getSourceFile() === file) ?? false);
}
