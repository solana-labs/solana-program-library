import * as ts from "typescript";
import type { Context } from "../converter";
import type { Reflection } from "./reflections/abstract";
import type { DeclarationReflection } from "./reflections/declaration";
import type { ProjectReflection } from "./reflections/project";
import type { Serializer, JSONOutput, Deserializer } from "../serialization";
import { ReflectionSymbolId } from "./reflections/ReflectionSymbolId";
import type { DeclarationReference } from "../converter/comments/declarationReference";
/**
 * Base class of all type definitions.
 */
export declare abstract class Type {
    /**
     * The type name identifier.
     */
    abstract readonly type: keyof TypeKindMap;
    /**
     * Return a string representation of this type.
     */
    toString(): string;
    /**
     * Visit this type, returning the value returned by the visitor.
     */
    visit<T>(visitor: TypeVisitor<T>): T;
    visit<T>(visitor: Partial<TypeVisitor<T>>): T | undefined;
    stringify(context: TypeContext): string;
    abstract toObject(serializer: Serializer): JSONOutput.SomeType;
    fromObject(_de: Deserializer, _obj: JSONOutput.SomeType): void;
    abstract needsParenthesis(context: TypeContext): boolean;
    /**
     * Implementation method for `toString`. `needsParenthesis` will be used to determine if
     * the returned string should be wrapped in parenthesis.
     */
    protected abstract getTypeString(): string;
}
export interface TypeKindMap {
    array: ArrayType;
    conditional: ConditionalType;
    indexedAccess: IndexedAccessType;
    inferred: InferredType;
    intersection: IntersectionType;
    intrinsic: IntrinsicType;
    literal: LiteralType;
    mapped: MappedType;
    optional: OptionalType;
    predicate: PredicateType;
    query: QueryType;
    reference: ReferenceType;
    reflection: ReflectionType;
    rest: RestType;
    templateLiteral: TemplateLiteralType;
    tuple: TupleType;
    namedTupleMember: NamedTupleMember;
    typeOperator: TypeOperatorType;
    union: UnionType;
    unknown: UnknownType;
}
export type TypeVisitor<T = void> = {
    [K in TypeKind]: (type: TypeKindMap[K]) => T;
};
export declare function makeRecursiveVisitor(visitor: Partial<TypeVisitor>): TypeVisitor;
export type TypeKind = keyof TypeKindMap;
export type SomeType = TypeKindMap[keyof TypeKindMap];
/**
 * Enumeration that can be used when traversing types to track the location of recursion.
 * Used by TypeDoc internally to track when to output parenthesis when rendering.
 * @enum
 */
export declare const TypeContext: {
    readonly none: "none";
    readonly templateLiteralElement: "templateLiteralElement";
    readonly arrayElement: "arrayElement";
    readonly indexedAccessElement: "indexedAccessElement";
    readonly conditionalCheck: "conditionalCheck";
    readonly conditionalExtends: "conditionalExtends";
    readonly conditionalTrue: "conditionalTrue";
    readonly conditionalFalse: "conditionalFalse";
    readonly indexedIndex: "indexedIndex";
    readonly indexedObject: "indexedObject";
    readonly inferredConstraint: "inferredConstraint";
    readonly intersectionElement: "intersectionElement";
    readonly mappedName: "mappedName";
    readonly mappedParameter: "mappedParameter";
    readonly mappedTemplate: "mappedTemplate";
    readonly optionalElement: "optionalElement";
    readonly predicateTarget: "predicateTarget";
    readonly queryTypeTarget: "queryTypeTarget";
    readonly typeOperatorTarget: "typeOperatorTarget";
    readonly referenceTypeArgument: "referenceTypeArgument";
    readonly restElement: "restElement";
    readonly tupleElement: "tupleElement";
    readonly unionElement: "unionElement";
};
export type TypeContext = (typeof TypeContext)[keyof typeof TypeContext];
/**
 * Represents an array type.
 *
 * ```ts
 * let value: string[];
 * ```
 */
export declare class ArrayType extends Type {
    elementType: SomeType;
    readonly type = "array";
    /**
     * @param elementType The type of the elements in the array.
     */
    constructor(elementType: SomeType);
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.ArrayType;
}
/**
 * Represents a conditional type.
 *
 * ```ts
 * let value: Check extends Extends ? True : False;
 * ```
 */
export declare class ConditionalType extends Type {
    checkType: SomeType;
    extendsType: SomeType;
    trueType: SomeType;
    falseType: SomeType;
    readonly type = "conditional";
    constructor(checkType: SomeType, extendsType: SomeType, trueType: SomeType, falseType: SomeType);
    protected getTypeString(): string;
    needsParenthesis(context: TypeContext): boolean;
    toObject(serializer: Serializer): JSONOutput.ConditionalType;
}
/**
 * Represents an indexed access type.
 */
export declare class IndexedAccessType extends Type {
    objectType: SomeType;
    indexType: SomeType;
    readonly type = "indexedAccess";
    constructor(objectType: SomeType, indexType: SomeType);
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.IndexedAccessType;
}
/**
 * Represents an inferred type, U in the example below.
 *
 * ```ts
 * type Z = Promise<string> extends Promise<infer U> : never
 * ```
 */
export declare class InferredType extends Type {
    name: string;
    constraint?: SomeType | undefined;
    readonly type = "inferred";
    constructor(name: string, constraint?: SomeType | undefined);
    protected getTypeString(): string;
    needsParenthesis(context: TypeContext): boolean;
    toObject(serializer: Serializer): JSONOutput.InferredType;
}
/**
 * Represents an intersection type.
 *
 * ```ts
 * let value: A & B;
 * ```
 */
export declare class IntersectionType extends Type {
    types: SomeType[];
    readonly type = "intersection";
    constructor(types: SomeType[]);
    protected getTypeString(): string;
    needsParenthesis(context: TypeContext): boolean;
    toObject(serializer: Serializer): JSONOutput.IntersectionType;
}
/**
 * Represents an intrinsic type like `string` or `boolean`.
 *
 * ```ts
 * let value: number;
 * ```
 */
export declare class IntrinsicType extends Type {
    name: string;
    readonly type = "intrinsic";
    constructor(name: string);
    protected getTypeString(): string;
    toObject(): JSONOutput.IntrinsicType;
    needsParenthesis(): boolean;
}
/**
 * Represents a literal type.
 *
 * ```ts
 * type A = "A"
 * type B = 1
 * ```
 */
export declare class LiteralType extends Type {
    value: string | number | boolean | null | bigint;
    readonly type = "literal";
    constructor(value: string | number | boolean | null | bigint);
    /**
     * Return a string representation of this type.
     */
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(): JSONOutput.LiteralType;
}
/**
 * Represents a mapped type.
 *
 * ```ts
 * { -readonly [K in Parameter as Name]?: Template }
 * ```
 */
export declare class MappedType extends Type {
    parameter: string;
    parameterType: SomeType;
    templateType: SomeType;
    readonlyModifier?: "+" | "-" | undefined;
    optionalModifier?: "+" | "-" | undefined;
    nameType?: SomeType | undefined;
    readonly type = "mapped";
    constructor(parameter: string, parameterType: SomeType, templateType: SomeType, readonlyModifier?: "+" | "-" | undefined, optionalModifier?: "+" | "-" | undefined, nameType?: SomeType | undefined);
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.MappedType;
}
/**
 * Represents an optional type
 * ```ts
 * type Z = [1, 2?]
 * //           ^^
 * ```
 */
export declare class OptionalType extends Type {
    elementType: SomeType;
    readonly type = "optional";
    constructor(elementType: SomeType);
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.OptionalType;
}
/**
 * Represents a type predicate.
 *
 * ```ts
 * function isString(x: unknown): x is string {}
 * function assert(condition: boolean): asserts condition {}
 * ```
 */
export declare class PredicateType extends Type {
    name: string;
    asserts: boolean;
    targetType?: SomeType | undefined;
    readonly type = "predicate";
    /**
     * Create a new PredicateType instance.
     *
     * @param name The identifier name which is tested by the predicate.
     * @param asserts True if the type is of the form `asserts val is string`,
     *                false if the type is of the form `val is string`
     * @param targetType The type that the identifier is tested to be.
     *                   May be undefined if the type is of the form `asserts val`.
     *                   Will be defined if the type is of the form `asserts val is string` or `val is string`.
     */
    constructor(name: string, asserts: boolean, targetType?: SomeType | undefined);
    /**
     * Return a string representation of this type.
     */
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.PredicateType;
}
/**
 * Represents a type that is constructed by querying the type of a reflection.
 * ```ts
 * const x = 1
 * type Z = typeof x // query on reflection for x
 * ```
 */
export declare class QueryType extends Type {
    queryType: ReferenceType;
    readonly type = "query";
    constructor(queryType: ReferenceType);
    protected getTypeString(): string;
    /**
     * @privateRemarks
     * An argument could be made that this ought to return true for indexedObject
     * since precedence is different than on the value side... if someone really cares
     * they can easily use a custom theme to change this.
     */
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.QueryType;
}
/**
 * Represents a type that refers to another reflection like a class, interface or enum.
 *
 * ```ts
 * let value: MyClass<T>;
 * ```
 */
export declare class ReferenceType extends Type {
    readonly type = "reference";
    /**
     * The name of the referenced type.
     *
     * If the symbol cannot be found because it's not part of the documentation this
     * can be used to represent the type.
     */
    name: string;
    /**
     * The type arguments of this reference.
     */
    typeArguments?: SomeType[];
    /**
     * The resolved reflection.
     */
    get reflection(): Reflection | undefined;
    /**
     * If not resolved, the symbol id of the reflection, otherwise undefined.
     */
    get symbolId(): ReflectionSymbolId | undefined;
    /**
     * Checks if this type is a reference type because it uses a name, but is intentionally not pointing
     * to a reflection. This happens for type parameters and when representing a mapped type.
     */
    isIntentionallyBroken(): boolean;
    /**
     * Convert this reference type to a declaration reference used for resolution of external types.
     */
    toDeclarationReference(): DeclarationReference;
    /**
     * The fully qualified name of the referenced type, relative to the file it is defined in.
     * This will usually be the same as `name`, unless namespaces are used.
     */
    qualifiedName: string;
    /**
     * The package that this type is referencing.
     */
    package?: string;
    /**
     * If this reference type refers to a reflection defined by a project not being rendered,
     * points to the url that this type should be linked to.
     */
    externalUrl?: string;
    /**
     * If set, no warnings about something not being exported should be created
     * since this may be referring to a type created with `infer X` which will not
     * be registered on the project.
     */
    refersToTypeParameter: boolean;
    private _target;
    private _project;
    private constructor();
    static createResolvedReference(name: string, target: Reflection | number, project: ProjectReflection | null): ReferenceType;
    static createSymbolReference(symbol: ts.Symbol, context: Context, name?: string): ReferenceType;
    /**
     * This is used for type parameters, which don't actually point to something,
     * and also for temporary references which will be cleaned up with real references
     * later during conversion.
     * @internal
     */
    static createBrokenReference(name: string, project: ProjectReflection): ReferenceType;
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.ReferenceType;
    fromObject(de: Deserializer, obj: JSONOutput.ReferenceType): void;
}
/**
 * Represents a type which has it's own reflection like literal types.
 * This type will likely go away at some point and be replaced by a dedicated
 * `ObjectType`. Allowing reflections to be nested within types causes much
 * pain in the rendering code.
 *
 * ```ts
 * let value: { a: string, b: number };
 * ```
 */
export declare class ReflectionType extends Type {
    declaration: DeclarationReflection;
    readonly type = "reflection";
    constructor(declaration: DeclarationReflection);
    protected getTypeString(): "Object" | "Function";
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.ReflectionType;
}
/**
 * Represents a rest type
 * ```ts
 * type Z = [1, ...2[]]
 * //           ^^^^^^
 * ```
 */
export declare class RestType extends Type {
    elementType: SomeType;
    readonly type = "rest";
    constructor(elementType: SomeType);
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.RestType;
}
/**
 * TS 4.1 template literal types
 * ```ts
 * type Z = `${'a' | 'b'}${'a' | 'b'}`
 * ```
 */
export declare class TemplateLiteralType extends Type {
    head: string;
    tail: [SomeType, string][];
    readonly type = "templateLiteral";
    constructor(head: string, tail: [SomeType, string][]);
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.TemplateLiteralType;
}
/**
 * Represents a tuple type.
 *
 * ```ts
 * let value: [string, boolean];
 * ```
 */
export declare class TupleType extends Type {
    elements: SomeType[];
    readonly type = "tuple";
    /**
     * @param elements The ordered type elements of the tuple type.
     */
    constructor(elements: SomeType[]);
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.TupleType;
}
/**
 * Represents a named member of a tuple type.
 *
 * ```ts
 * let value: [name: string];
 * ```
 */
export declare class NamedTupleMember extends Type {
    name: string;
    isOptional: boolean;
    element: SomeType;
    readonly type = "namedTupleMember";
    constructor(name: string, isOptional: boolean, element: SomeType);
    /**
     * Return a string representation of this type.
     */
    protected getTypeString(): string;
    needsParenthesis(): boolean;
    toObject(serializer: Serializer): JSONOutput.NamedTupleMemberType;
}
/**
 * Represents a type operator type.
 *
 * ```ts
 * class A {}
 * class B<T extends keyof A> {}
 * ```
 */
export declare class TypeOperatorType extends Type {
    target: SomeType;
    operator: "keyof" | "unique" | "readonly";
    readonly type = "typeOperator";
    constructor(target: SomeType, operator: "keyof" | "unique" | "readonly");
    protected getTypeString(): string;
    needsParenthesis(context: TypeContext): boolean;
    toObject(serializer: Serializer): JSONOutput.TypeOperatorType;
}
/**
 * Represents an union type.
 *
 * ```ts
 * let value: string | string[];
 * ```
 */
export declare class UnionType extends Type {
    types: SomeType[];
    readonly type = "union";
    constructor(types: SomeType[]);
    protected getTypeString(): string;
    needsParenthesis(context: TypeContext): boolean;
    private normalize;
    toObject(serializer: Serializer): JSONOutput.UnionType;
}
/**
 * Represents all unknown types that cannot be converted by TypeDoc.
 */
export declare class UnknownType extends Type {
    readonly type = "unknown";
    /**
     * A string representation of the type as returned from TypeScript compiler.
     */
    name: string;
    constructor(name: string);
    protected getTypeString(): string;
    /**
     * Always returns true if not at the root level, we have no idea what's in here, so wrap it in parenthesis
     * to be extra safe.
     */
    needsParenthesis(context: TypeContext): boolean;
    toObject(): JSONOutput.UnknownType;
}
