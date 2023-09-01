import ts from "typescript";
import { Reflection, ProjectReflection, DeclarationReflection, ReflectionKind } from "../models/index";
import type { Converter } from "./converter";
/**
 * The context describes the current state the converter is in.
 */
export declare class Context {
    /**
     * The converter instance that has created the context.
     */
    readonly converter: Converter;
    /**
     * The TypeChecker instance returned by the TypeScript compiler.
     */
    get checker(): ts.TypeChecker;
    /**
     * The program currently being converted.
     * Accessing this property will throw if a source file is not currently being converted.
     */
    get program(): ts.Program;
    private _program?;
    /**
     * All programs being converted.
     */
    readonly programs: readonly ts.Program[];
    /**
     * The project that is currently processed.
     */
    readonly project: ProjectReflection;
    /**
     * The scope or parent reflection that is currently processed.
     */
    readonly scope: Reflection;
    convertingTypeNode: boolean;
    convertingClassOrInterface: boolean;
    shouldBeStatic: boolean;
    /**
     * Create a new Context instance.
     *
     * @param converter  The converter instance that has created the context.
     * @internal
     */
    constructor(converter: Converter, programs: readonly ts.Program[], project: ProjectReflection, scope?: Context["scope"]);
    /** @internal */
    get logger(): import("../utils").Logger;
    /**
     * Return the type declaration of the given node.
     *
     * @param node  The TypeScript node whose type should be resolved.
     * @returns The type declaration of the given node.
     */
    getTypeAtLocation(node: ts.Node): ts.Type | undefined;
    getSymbolAtLocation(node: ts.Node): ts.Symbol | undefined;
    expectSymbolAtLocation(node: ts.Node): ts.Symbol;
    resolveAliasedSymbol(symbol: ts.Symbol): ts.Symbol;
    createDeclarationReflection(kind: ReflectionKind, symbol: ts.Symbol | undefined, exportSymbol: ts.Symbol | undefined, nameOverride?: string): DeclarationReflection;
    postReflectionCreation(reflection: Reflection, symbol: ts.Symbol | undefined, exportSymbol: ts.Symbol | undefined): void;
    finalizeDeclarationReflection(reflection: DeclarationReflection): void;
    addChild(reflection: DeclarationReflection): void;
    shouldIgnore(symbol: ts.Symbol): boolean;
    /**
     * Register a newly generated reflection. All created reflections should be
     * passed to this method to ensure that the project helper functions work correctly.
     *
     * @param reflection  The reflection that should be registered.
     * @param symbol  The symbol the given reflection was resolved from.
     */
    registerReflection(reflection: Reflection, symbol: ts.Symbol | undefined): void;
    /**
     * Trigger a node reflection event.
     *
     * All events are dispatched on the current converter instance.
     *
     * @param name  The name of the event that should be triggered.
     * @param reflection  The triggering reflection.
     * @param node  The triggering TypeScript node if available.
     */
    trigger(name: string, reflection: Reflection, node?: ts.Node): void;
    /** @internal */
    setActiveProgram(program: ts.Program | undefined): void;
    getComment(symbol: ts.Symbol, kind: ReflectionKind): import("../models/index").Comment | undefined;
    getFileComment(node: ts.SourceFile): import("../models/index").Comment | undefined;
    getJsDocComment(declaration: ts.JSDocPropertyLikeTag | ts.JSDocCallbackTag | ts.JSDocTypedefTag | ts.JSDocTemplateTag | ts.JSDocEnumTag): import("../models/index").Comment | undefined;
    getSignatureComment(declaration: ts.SignatureDeclaration | ts.JSDocSignature): import("../models/index").Comment | undefined;
    /**
     * @param callback  The callback function that should be executed with the changed context.
     */
    withScope(scope: Reflection): Context;
}
