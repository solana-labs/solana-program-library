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
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.UnknownType = exports.UnionType = exports.TypeOperatorType = exports.NamedTupleMember = exports.TupleType = exports.TemplateLiteralType = exports.RestType = exports.ReflectionType = exports.ReferenceType = exports.QueryType = exports.PredicateType = exports.OptionalType = exports.MappedType = exports.LiteralType = exports.IntrinsicType = exports.IntersectionType = exports.InferredType = exports.IndexedAccessType = exports.ConditionalType = exports.ArrayType = exports.TypeContext = exports.makeRecursiveVisitor = exports.Type = void 0;
const ts = __importStar(require("typescript"));
const tsutils_1 = require("../utils/tsutils");
const ReflectionSymbolId_1 = require("./reflections/ReflectionSymbolId");
const fs_1 = require("../utils/fs");
/**
 * Base class of all type definitions.
 */
class Type {
    /**
     * Return a string representation of this type.
     */
    toString() {
        return this.stringify(exports.TypeContext.none);
    }
    visit(visitor) {
        return visitor[this.type]?.(this);
    }
    stringify(context) {
        if (this.needsParenthesis(context)) {
            return `(${this.getTypeString()})`;
        }
        return this.getTypeString();
    }
    // Nothing to do for the majority of types.
    fromObject(_de, _obj) { }
}
exports.Type = Type;
function makeRecursiveVisitor(visitor) {
    const recursiveVisitor = {
        namedTupleMember(type) {
            visitor.namedTupleMember?.(type);
            type.element.visit(recursiveVisitor);
        },
        templateLiteral(type) {
            visitor.templateLiteral?.(type);
            for (const [h] of type.tail) {
                h.visit(recursiveVisitor);
            }
        },
        array(type) {
            visitor.array?.(type);
            type.elementType.visit(recursiveVisitor);
        },
        conditional(type) {
            visitor.conditional?.(type);
            type.checkType.visit(recursiveVisitor);
            type.extendsType.visit(recursiveVisitor);
            type.trueType.visit(recursiveVisitor);
            type.falseType.visit(recursiveVisitor);
        },
        indexedAccess(type) {
            visitor.indexedAccess?.(type);
            type.indexType.visit(recursiveVisitor);
            type.objectType.visit(recursiveVisitor);
        },
        inferred(type) {
            visitor.inferred?.(type);
            type.constraint?.visit(recursiveVisitor);
        },
        intersection(type) {
            visitor.intersection?.(type);
            type.types.forEach((t) => t.visit(recursiveVisitor));
        },
        intrinsic(type) {
            visitor.intrinsic?.(type);
        },
        literal(type) {
            visitor.literal?.(type);
        },
        mapped(type) {
            visitor.mapped?.(type);
            type.nameType?.visit(recursiveVisitor);
            type.parameterType.visit(recursiveVisitor);
            type.templateType.visit(recursiveVisitor);
        },
        optional(type) {
            visitor.optional?.(type);
            type.elementType.visit(recursiveVisitor);
        },
        predicate(type) {
            visitor.predicate?.(type);
            type.targetType?.visit(recursiveVisitor);
        },
        query(type) {
            visitor.query?.(type);
            type.queryType.visit(recursiveVisitor);
        },
        reference(type) {
            visitor.reference?.(type);
            type.typeArguments?.forEach((t) => t.visit(recursiveVisitor));
        },
        reflection(type) {
            visitor.reflection?.(type);
            // Future: This should maybe recurse too?
            // See the validator in exports.ts for how to do it.
        },
        rest(type) {
            visitor.rest?.(type);
            type.elementType.visit(recursiveVisitor);
        },
        tuple(type) {
            visitor.tuple?.(type);
            type.elements.forEach((t) => t.visit(recursiveVisitor));
        },
        typeOperator(type) {
            visitor.typeOperator?.(type);
            type.target.visit(recursiveVisitor);
        },
        union(type) {
            visitor.union?.(type);
            type.types.forEach((t) => t.visit(recursiveVisitor));
        },
        unknown(type) {
            visitor.unknown?.(type);
        },
    };
    return recursiveVisitor;
}
exports.makeRecursiveVisitor = makeRecursiveVisitor;
/**
 * Enumeration that can be used when traversing types to track the location of recursion.
 * Used by TypeDoc internally to track when to output parenthesis when rendering.
 * @enum
 */
exports.TypeContext = {
    none: "none",
    templateLiteralElement: "templateLiteralElement",
    arrayElement: "arrayElement",
    indexedAccessElement: "indexedAccessElement",
    conditionalCheck: "conditionalCheck",
    conditionalExtends: "conditionalExtends",
    conditionalTrue: "conditionalTrue",
    conditionalFalse: "conditionalFalse",
    indexedIndex: "indexedIndex",
    indexedObject: "indexedObject",
    inferredConstraint: "inferredConstraint",
    intersectionElement: "intersectionElement",
    mappedName: "mappedName",
    mappedParameter: "mappedParameter",
    mappedTemplate: "mappedTemplate",
    optionalElement: "optionalElement",
    predicateTarget: "predicateTarget",
    queryTypeTarget: "queryTypeTarget",
    typeOperatorTarget: "typeOperatorTarget",
    referenceTypeArgument: "referenceTypeArgument",
    restElement: "restElement",
    tupleElement: "tupleElement",
    unionElement: "unionElement", // here | 1
};
/**
 * Represents an array type.
 *
 * ```ts
 * let value: string[];
 * ```
 */
class ArrayType extends Type {
    /**
     * @param elementType The type of the elements in the array.
     */
    constructor(elementType) {
        super();
        this.elementType = elementType;
        this.type = "array";
    }
    getTypeString() {
        return this.elementType.stringify(exports.TypeContext.arrayElement) + "[]";
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            elementType: serializer.toObject(this.elementType),
        };
    }
}
exports.ArrayType = ArrayType;
/**
 * Represents a conditional type.
 *
 * ```ts
 * let value: Check extends Extends ? True : False;
 * ```
 */
class ConditionalType extends Type {
    constructor(checkType, extendsType, trueType, falseType) {
        super();
        this.checkType = checkType;
        this.extendsType = extendsType;
        this.trueType = trueType;
        this.falseType = falseType;
        this.type = "conditional";
    }
    getTypeString() {
        return [
            this.checkType.stringify(exports.TypeContext.conditionalCheck),
            "extends",
            this.extendsType.stringify(exports.TypeContext.conditionalExtends),
            "?",
            this.trueType.stringify(exports.TypeContext.conditionalTrue),
            ":",
            this.falseType.stringify(exports.TypeContext.conditionalFalse),
        ].join(" ");
    }
    needsParenthesis(context) {
        const map = {
            none: false,
            templateLiteralElement: false,
            arrayElement: true,
            indexedAccessElement: false,
            conditionalCheck: true,
            conditionalExtends: true,
            conditionalTrue: false,
            conditionalFalse: false,
            indexedIndex: false,
            indexedObject: true,
            inferredConstraint: true,
            intersectionElement: true,
            mappedName: false,
            mappedParameter: false,
            mappedTemplate: false,
            optionalElement: true,
            predicateTarget: false,
            queryTypeTarget: false,
            typeOperatorTarget: true,
            referenceTypeArgument: false,
            restElement: true,
            tupleElement: false,
            unionElement: true,
        };
        return map[context];
    }
    toObject(serializer) {
        return {
            type: this.type,
            checkType: serializer.toObject(this.checkType),
            extendsType: serializer.toObject(this.extendsType),
            trueType: serializer.toObject(this.trueType),
            falseType: serializer.toObject(this.falseType),
        };
    }
}
exports.ConditionalType = ConditionalType;
/**
 * Represents an indexed access type.
 */
class IndexedAccessType extends Type {
    constructor(objectType, indexType) {
        super();
        this.objectType = objectType;
        this.indexType = indexType;
        this.type = "indexedAccess";
    }
    getTypeString() {
        return [
            this.objectType.stringify(exports.TypeContext.indexedObject),
            "[",
            this.indexType.stringify(exports.TypeContext.indexedIndex),
            "]",
        ].join("");
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            indexType: serializer.toObject(this.indexType),
            objectType: serializer.toObject(this.objectType),
        };
    }
}
exports.IndexedAccessType = IndexedAccessType;
/**
 * Represents an inferred type, U in the example below.
 *
 * ```ts
 * type Z = Promise<string> extends Promise<infer U> : never
 * ```
 */
class InferredType extends Type {
    constructor(name, constraint) {
        super();
        this.name = name;
        this.constraint = constraint;
        this.type = "inferred";
    }
    getTypeString() {
        if (this.constraint) {
            return `infer ${this.name} extends ${this.constraint.stringify(exports.TypeContext.inferredConstraint)}`;
        }
        return `infer ${this.name}`;
    }
    needsParenthesis(context) {
        const map = {
            none: false,
            templateLiteralElement: false,
            arrayElement: true,
            indexedAccessElement: false,
            conditionalCheck: false,
            conditionalExtends: false,
            conditionalTrue: false,
            conditionalFalse: false,
            indexedIndex: false,
            indexedObject: true,
            inferredConstraint: false,
            intersectionElement: false,
            mappedName: false,
            mappedParameter: false,
            mappedTemplate: false,
            optionalElement: true,
            predicateTarget: false,
            queryTypeTarget: false,
            typeOperatorTarget: false,
            referenceTypeArgument: false,
            restElement: true,
            tupleElement: false,
            unionElement: false,
        };
        return map[context];
    }
    toObject(serializer) {
        return {
            type: this.type,
            name: this.name,
            constraint: serializer.toObject(this.constraint),
        };
    }
}
exports.InferredType = InferredType;
/**
 * Represents an intersection type.
 *
 * ```ts
 * let value: A & B;
 * ```
 */
class IntersectionType extends Type {
    constructor(types) {
        super();
        this.types = types;
        this.type = "intersection";
    }
    getTypeString() {
        return this.types
            .map((t) => t.stringify(exports.TypeContext.intersectionElement))
            .join(" & ");
    }
    needsParenthesis(context) {
        const map = {
            none: false,
            templateLiteralElement: false,
            arrayElement: true,
            indexedAccessElement: false,
            conditionalCheck: true,
            conditionalExtends: false,
            conditionalTrue: false,
            conditionalFalse: false,
            indexedIndex: false,
            indexedObject: true,
            inferredConstraint: false,
            intersectionElement: false,
            mappedName: false,
            mappedParameter: false,
            mappedTemplate: false,
            optionalElement: true,
            predicateTarget: false,
            queryTypeTarget: false,
            typeOperatorTarget: true,
            referenceTypeArgument: false,
            restElement: true,
            tupleElement: false,
            unionElement: false,
        };
        return map[context];
    }
    toObject(serializer) {
        return {
            type: this.type,
            types: this.types.map((t) => serializer.toObject(t)),
        };
    }
}
exports.IntersectionType = IntersectionType;
/**
 * Represents an intrinsic type like `string` or `boolean`.
 *
 * ```ts
 * let value: number;
 * ```
 */
class IntrinsicType extends Type {
    constructor(name) {
        super();
        this.name = name;
        this.type = "intrinsic";
    }
    getTypeString() {
        return this.name;
    }
    toObject() {
        return {
            type: this.type,
            name: this.name,
        };
    }
    needsParenthesis() {
        return false;
    }
}
exports.IntrinsicType = IntrinsicType;
/**
 * Represents a literal type.
 *
 * ```ts
 * type A = "A"
 * type B = 1
 * ```
 */
class LiteralType extends Type {
    constructor(value) {
        super();
        this.value = value;
        this.type = "literal";
    }
    /**
     * Return a string representation of this type.
     */
    getTypeString() {
        if (typeof this.value === "bigint") {
            return this.value.toString() + "n";
        }
        return JSON.stringify(this.value);
    }
    needsParenthesis() {
        return false;
    }
    toObject() {
        if (typeof this.value === "bigint") {
            return {
                type: this.type,
                value: {
                    value: this.value.toString().replace("-", ""),
                    negative: this.value < BigInt("0"),
                },
            };
        }
        return {
            type: this.type,
            value: this.value,
        };
    }
}
exports.LiteralType = LiteralType;
/**
 * Represents a mapped type.
 *
 * ```ts
 * { -readonly [K in Parameter as Name]?: Template }
 * ```
 */
class MappedType extends Type {
    constructor(parameter, parameterType, templateType, readonlyModifier, optionalModifier, nameType) {
        super();
        this.parameter = parameter;
        this.parameterType = parameterType;
        this.templateType = templateType;
        this.readonlyModifier = readonlyModifier;
        this.optionalModifier = optionalModifier;
        this.nameType = nameType;
        this.type = "mapped";
    }
    getTypeString() {
        const read = {
            "+": "readonly ",
            "-": "-readonly ",
            "": "",
        }[this.readonlyModifier ?? ""];
        const opt = {
            "+": "?",
            "-": "-?",
            "": "",
        }[this.optionalModifier ?? ""];
        const parts = [
            "{ ",
            read,
            "[",
            this.parameter,
            " in ",
            this.parameterType.stringify(exports.TypeContext.mappedParameter),
        ];
        if (this.nameType) {
            parts.push(" as ", this.nameType.stringify(exports.TypeContext.mappedName));
        }
        parts.push("]", opt, ": ", this.templateType.stringify(exports.TypeContext.mappedTemplate), " }");
        return parts.join("");
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            parameter: this.parameter,
            parameterType: serializer.toObject(this.parameterType),
            templateType: serializer.toObject(this.templateType),
            readonlyModifier: this.readonlyModifier,
            optionalModifier: this.optionalModifier,
            nameType: serializer.toObject(this.nameType),
        };
    }
}
exports.MappedType = MappedType;
/**
 * Represents an optional type
 * ```ts
 * type Z = [1, 2?]
 * //           ^^
 * ```
 */
class OptionalType extends Type {
    constructor(elementType) {
        super();
        this.elementType = elementType;
        this.type = "optional";
    }
    getTypeString() {
        return this.elementType.stringify(exports.TypeContext.optionalElement) + "?";
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            elementType: serializer.toObject(this.elementType),
        };
    }
}
exports.OptionalType = OptionalType;
/**
 * Represents a type predicate.
 *
 * ```ts
 * function isString(x: unknown): x is string {}
 * function assert(condition: boolean): asserts condition {}
 * ```
 */
class PredicateType extends Type {
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
    constructor(name, asserts, targetType) {
        super();
        this.name = name;
        this.asserts = asserts;
        this.targetType = targetType;
        this.type = "predicate";
    }
    /**
     * Return a string representation of this type.
     */
    getTypeString() {
        const out = this.asserts ? ["asserts", this.name] : [this.name];
        if (this.targetType) {
            out.push("is", this.targetType.stringify(exports.TypeContext.predicateTarget));
        }
        return out.join(" ");
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            name: this.name,
            asserts: this.asserts,
            targetType: serializer.toObject(this.targetType),
        };
    }
}
exports.PredicateType = PredicateType;
/**
 * Represents a type that is constructed by querying the type of a reflection.
 * ```ts
 * const x = 1
 * type Z = typeof x // query on reflection for x
 * ```
 */
class QueryType extends Type {
    constructor(queryType) {
        super();
        this.queryType = queryType;
        this.type = "query";
    }
    getTypeString() {
        return `typeof ${this.queryType.stringify(exports.TypeContext.queryTypeTarget)}`;
    }
    /**
     * @privateRemarks
     * An argument could be made that this ought to return true for indexedObject
     * since precedence is different than on the value side... if someone really cares
     * they can easily use a custom theme to change this.
     */
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            queryType: serializer.toObject(this.queryType),
        };
    }
}
exports.QueryType = QueryType;
/**
 * Represents a type that refers to another reflection like a class, interface or enum.
 *
 * ```ts
 * let value: MyClass<T>;
 * ```
 */
class ReferenceType extends Type {
    /**
     * The resolved reflection.
     */
    get reflection() {
        if (typeof this._target === "number") {
            return this._project?.getReflectionById(this._target);
        }
        const resolved = this._project?.getReflectionFromSymbolId(this._target);
        if (resolved)
            this._target = resolved.id;
        return resolved;
    }
    /**
     * If not resolved, the symbol id of the reflection, otherwise undefined.
     */
    get symbolId() {
        if (!this.reflection && typeof this._target === "object") {
            return this._target;
        }
    }
    /**
     * Checks if this type is a reference type because it uses a name, but is intentionally not pointing
     * to a reflection. This happens for type parameters and when representing a mapped type.
     */
    isIntentionallyBroken() {
        return this._target === -1 || this.refersToTypeParameter;
    }
    /**
     * Convert this reference type to a declaration reference used for resolution of external types.
     */
    toDeclarationReference() {
        return {
            resolutionStart: "global",
            moduleSource: this.package,
            symbolReference: {
                path: this.qualifiedName
                    .split(".")
                    .map((p) => ({ path: p, navigation: "." })),
            },
        };
    }
    constructor(name, target, project, qualifiedName) {
        super();
        this.type = "reference";
        /**
         * If set, no warnings about something not being exported should be created
         * since this may be referring to a type created with `infer X` which will not
         * be registered on the project.
         */
        this.refersToTypeParameter = false;
        this.name = name;
        if (typeof target === "number") {
            this._target = target;
        }
        else {
            this._target = "variant" in target ? target.id : target;
        }
        this._project = project;
        this.qualifiedName = qualifiedName;
    }
    static createResolvedReference(name, target, project) {
        return new ReferenceType(name, target, project, name);
    }
    static createSymbolReference(symbol, context, name) {
        // Type parameters should never have resolved references because they
        // cannot be linked to, and might be declared within the type with conditional types.
        if (symbol.flags & ts.SymbolFlags.TypeParameter) {
            const ref = ReferenceType.createBrokenReference(name ?? symbol.name, context.project);
            ref.refersToTypeParameter = true;
            return ref;
        }
        const ref = new ReferenceType(name ?? symbol.name, new ReflectionSymbolId_1.ReflectionSymbolId(symbol), context.project, (0, tsutils_1.getQualifiedName)(symbol, name ?? symbol.name));
        const symbolPath = symbol?.declarations?.[0]
            ?.getSourceFile()
            .fileName.replace(/\\/g, "/");
        if (!symbolPath)
            return ref;
        // Attempt to decide package name from path if it contains "node_modules"
        let startIndex = symbolPath.lastIndexOf("node_modules/");
        if (startIndex !== -1) {
            startIndex += "node_modules/".length;
            let stopIndex = symbolPath.indexOf("/", startIndex);
            // Scoped package, e.g. `@types/node`
            if (symbolPath[startIndex] === "@") {
                stopIndex = symbolPath.indexOf("/", stopIndex + 1);
            }
            const packageName = symbolPath.substring(startIndex, stopIndex);
            ref.package = packageName;
            return ref;
        }
        // Otherwise, look for a "package.json" file in a parent path
        ref.package = (0, fs_1.findPackageForPath)(symbolPath);
        return ref;
    }
    /**
     * This is used for type parameters, which don't actually point to something,
     * and also for temporary references which will be cleaned up with real references
     * later during conversion.
     * @internal
     */
    static createBrokenReference(name, project) {
        return new ReferenceType(name, -1, project, name);
    }
    getTypeString() {
        const name = this.reflection ? this.reflection.name : this.name;
        let typeArgs = "";
        if (this.typeArguments && this.typeArguments.length > 0) {
            typeArgs += "<";
            typeArgs += this.typeArguments
                .map((arg) => arg.stringify(exports.TypeContext.referenceTypeArgument))
                .join(", ");
            typeArgs += ">";
        }
        return name + typeArgs;
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        const result = {
            type: this.type,
            target: typeof this._target === "number"
                ? this._target
                : this._target.toObject(serializer),
            typeArguments: serializer.toObjectsOptional(this.typeArguments),
            name: this.name,
            package: this.package,
            externalUrl: this.externalUrl,
        };
        if (this.name !== this.qualifiedName) {
            result.qualifiedName = this.qualifiedName;
        }
        if (this.refersToTypeParameter) {
            result.refersToTypeParameter = true;
        }
        return result;
    }
    fromObject(de, obj) {
        this.typeArguments = de.reviveMany(obj.typeArguments, (t) => de.constructType(t));
        if (typeof obj.target === "number" && obj.target !== -1) {
            de.defer((project) => {
                const target = project.getReflectionById(de.oldIdToNewId[obj.target] ?? -1);
                if (target) {
                    this._project = project;
                    this._target = target.id;
                }
                else {
                    de.logger.warn(`Serialized project contained a reference to ${obj.target} (${this.qualifiedName}), which was not a part of the project.`);
                }
            });
        }
        else if (obj.target === -1) {
            this._target = -1;
        }
        else {
            this._project = de.project;
            this._target = new ReflectionSymbolId_1.ReflectionSymbolId(obj.target);
        }
        this.qualifiedName = obj.qualifiedName ?? obj.name;
        this.package = obj.package;
        this.refersToTypeParameter = !!obj.refersToTypeParameter;
    }
}
exports.ReferenceType = ReferenceType;
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
class ReflectionType extends Type {
    constructor(declaration) {
        super();
        this.declaration = declaration;
        this.type = "reflection";
    }
    // This really ought to do better, but I'm putting off investing effort here until
    // I'm fully convinced that keeping this is a good idea. Currently, I'd much rather
    // change object types to not create reflections.
    getTypeString() {
        if (!this.declaration.children && this.declaration.signatures) {
            return "Function";
        }
        else {
            return "Object";
        }
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            declaration: serializer.toObject(this.declaration),
        };
    }
}
exports.ReflectionType = ReflectionType;
/**
 * Represents a rest type
 * ```ts
 * type Z = [1, ...2[]]
 * //           ^^^^^^
 * ```
 */
class RestType extends Type {
    constructor(elementType) {
        super();
        this.elementType = elementType;
        this.type = "rest";
    }
    getTypeString() {
        return `...${this.elementType.stringify(exports.TypeContext.restElement)}`;
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            elementType: serializer.toObject(this.elementType),
        };
    }
}
exports.RestType = RestType;
/**
 * TS 4.1 template literal types
 * ```ts
 * type Z = `${'a' | 'b'}${'a' | 'b'}`
 * ```
 */
class TemplateLiteralType extends Type {
    constructor(head, tail) {
        super();
        this.head = head;
        this.tail = tail;
        this.type = "templateLiteral";
    }
    getTypeString() {
        return [
            "`",
            this.head,
            ...this.tail.map(([type, text]) => {
                return ("${" +
                    type.stringify(exports.TypeContext.templateLiteralElement) +
                    "}" +
                    text);
            }),
            "`",
        ].join("");
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            head: this.head,
            tail: this.tail.map(([type, text]) => [
                serializer.toObject(type),
                text,
            ]),
        };
    }
}
exports.TemplateLiteralType = TemplateLiteralType;
/**
 * Represents a tuple type.
 *
 * ```ts
 * let value: [string, boolean];
 * ```
 */
class TupleType extends Type {
    /**
     * @param elements The ordered type elements of the tuple type.
     */
    constructor(elements) {
        super();
        this.elements = elements;
        this.type = "tuple";
    }
    getTypeString() {
        return ("[" +
            this.elements
                .map((t) => t.stringify(exports.TypeContext.tupleElement))
                .join(", ") +
            "]");
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            elements: serializer.toObjectsOptional(this.elements),
        };
    }
}
exports.TupleType = TupleType;
/**
 * Represents a named member of a tuple type.
 *
 * ```ts
 * let value: [name: string];
 * ```
 */
class NamedTupleMember extends Type {
    constructor(name, isOptional, element) {
        super();
        this.name = name;
        this.isOptional = isOptional;
        this.element = element;
        this.type = "namedTupleMember";
    }
    /**
     * Return a string representation of this type.
     */
    getTypeString() {
        return `${this.name}${this.isOptional ? "?" : ""}: ${this.element.stringify(exports.TypeContext.tupleElement)}`;
    }
    needsParenthesis() {
        return false;
    }
    toObject(serializer) {
        return {
            type: this.type,
            name: this.name,
            isOptional: this.isOptional,
            element: serializer.toObject(this.element),
        };
    }
}
exports.NamedTupleMember = NamedTupleMember;
/**
 * Represents a type operator type.
 *
 * ```ts
 * class A {}
 * class B<T extends keyof A> {}
 * ```
 */
class TypeOperatorType extends Type {
    constructor(target, operator) {
        super();
        this.target = target;
        this.operator = operator;
        this.type = "typeOperator";
    }
    getTypeString() {
        return `${this.operator} ${this.target.stringify(exports.TypeContext.typeOperatorTarget)}`;
    }
    needsParenthesis(context) {
        const map = {
            none: false,
            templateLiteralElement: false,
            arrayElement: true,
            indexedAccessElement: false,
            conditionalCheck: false,
            conditionalExtends: false,
            conditionalTrue: false,
            conditionalFalse: false,
            indexedIndex: false,
            indexedObject: true,
            inferredConstraint: false,
            intersectionElement: false,
            mappedName: false,
            mappedParameter: false,
            mappedTemplate: false,
            optionalElement: true,
            predicateTarget: false,
            queryTypeTarget: false,
            typeOperatorTarget: false,
            referenceTypeArgument: false,
            restElement: false,
            tupleElement: false,
            unionElement: false,
        };
        return map[context];
    }
    toObject(serializer) {
        return {
            type: this.type,
            operator: this.operator,
            target: serializer.toObject(this.target),
        };
    }
}
exports.TypeOperatorType = TypeOperatorType;
/**
 * Represents an union type.
 *
 * ```ts
 * let value: string | string[];
 * ```
 */
class UnionType extends Type {
    constructor(types) {
        super();
        this.types = types;
        this.type = "union";
        this.normalize();
    }
    getTypeString() {
        return this.types
            .map((t) => t.stringify(exports.TypeContext.unionElement))
            .join(" | ");
    }
    needsParenthesis(context) {
        const map = {
            none: false,
            templateLiteralElement: false,
            arrayElement: true,
            indexedAccessElement: false,
            conditionalCheck: true,
            conditionalExtends: false,
            conditionalTrue: false,
            conditionalFalse: false,
            indexedIndex: false,
            indexedObject: true,
            inferredConstraint: false,
            intersectionElement: true,
            mappedName: false,
            mappedParameter: false,
            mappedTemplate: false,
            optionalElement: true,
            predicateTarget: false,
            queryTypeTarget: false,
            typeOperatorTarget: true,
            referenceTypeArgument: false,
            restElement: false,
            tupleElement: false,
            unionElement: false,
        };
        return map[context];
    }
    normalize() {
        let trueIndex = -1;
        let falseIndex = -1;
        for (let i = 0; i < this.types.length && (trueIndex === -1 || falseIndex === -1); i++) {
            const t = this.types[i];
            if (t instanceof LiteralType) {
                if (t.value === true) {
                    trueIndex = i;
                }
                if (t.value === false) {
                    falseIndex = i;
                }
            }
        }
        if (trueIndex !== -1 && falseIndex !== -1) {
            this.types.splice(Math.max(trueIndex, falseIndex), 1);
            this.types.splice(Math.min(trueIndex, falseIndex), 1, new IntrinsicType("boolean"));
        }
    }
    toObject(serializer) {
        return {
            type: this.type,
            types: this.types.map((t) => serializer.toObject(t)),
        };
    }
}
exports.UnionType = UnionType;
/**
 * Represents all unknown types that cannot be converted by TypeDoc.
 */
class UnknownType extends Type {
    constructor(name) {
        super();
        this.type = "unknown";
        this.name = name;
    }
    getTypeString() {
        return this.name;
    }
    /**
     * Always returns true if not at the root level, we have no idea what's in here, so wrap it in parenthesis
     * to be extra safe.
     */
    needsParenthesis(context) {
        return context !== exports.TypeContext.none;
    }
    toObject() {
        return {
            type: this.type,
            name: this.name,
        };
    }
}
exports.UnknownType = UnknownType;
