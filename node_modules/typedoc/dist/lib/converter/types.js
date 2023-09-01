"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.convertType = exports.loadConverters = void 0;
const assert_1 = __importDefault(require("assert"));
const typescript_1 = __importDefault(require("typescript"));
const models_1 = require("../models");
const ReflectionSymbolId_1 = require("../models/reflections/ReflectionSymbolId");
const array_1 = require("../utils/array");
const converter_events_1 = require("./converter-events");
const index_signature_1 = require("./factories/index-signature");
const signature_1 = require("./factories/signature");
const symbols_1 = require("./symbols");
const nodes_1 = require("./utils/nodes");
const reflections_1 = require("./utils/reflections");
const converters = new Map();
function loadConverters() {
    if (converters.size)
        return;
    for (const actor of [
        arrayConverter,
        conditionalConverter,
        constructorConverter,
        exprWithTypeArgsConverter,
        functionTypeConverter,
        importType,
        indexedAccessConverter,
        inferredConverter,
        intersectionConverter,
        jsDocVariadicTypeConverter,
        keywordConverter,
        optionalConverter,
        parensConverter,
        predicateConverter,
        queryConverter,
        typeLiteralConverter,
        referenceConverter,
        restConverter,
        namedTupleMemberConverter,
        mappedConverter,
        literalTypeConverter,
        templateLiteralConverter,
        thisConverter,
        tupleConverter,
        typeOperatorConverter,
        unionConverter,
        // Only used if skipLibCheck: true
        jsDocNullableTypeConverter,
        jsDocNonNullableTypeConverter,
    ]) {
        for (const key of actor.kind) {
            if (key === undefined) {
                // Might happen if running on an older TS version.
                continue;
            }
            (0, assert_1.default)(!converters.has(key));
            converters.set(key, actor);
        }
    }
}
exports.loadConverters = loadConverters;
// This ought not be necessary, but we need some way to discover recursively
// typed symbols which do not have type nodes. See the `recursive` symbol in the variables test.
const seenTypeSymbols = new Set();
function maybeConvertType(context, typeOrNode) {
    if (!typeOrNode) {
        return;
    }
    return convertType(context, typeOrNode);
}
function convertType(context, typeOrNode) {
    if (!typeOrNode) {
        return new models_1.IntrinsicType("any");
    }
    loadConverters();
    if ("kind" in typeOrNode) {
        const converter = converters.get(typeOrNode.kind);
        if (converter) {
            return converter.convert(context, typeOrNode);
        }
        return requestBugReport(context, typeOrNode);
    }
    // IgnoreErrors is important, without it, we can't assert that we will get a node.
    const node = context.checker.typeToTypeNode(typeOrNode, void 0, typescript_1.default.NodeBuilderFlags.IgnoreErrors);
    (0, assert_1.default)(node); // According to the TS source of typeToString, this is a bug if it does not hold.
    const symbol = typeOrNode.getSymbol();
    if (symbol) {
        if (node.kind !== typescript_1.default.SyntaxKind.TypeReference &&
            node.kind !== typescript_1.default.SyntaxKind.ArrayType &&
            seenTypeSymbols.has(symbol)) {
            const typeString = context.checker.typeToString(typeOrNode);
            context.logger.verbose(`Refusing to recurse when converting type: ${typeString}`);
            return new models_1.UnknownType(typeString);
        }
        seenTypeSymbols.add(symbol);
    }
    let converter = converters.get(node.kind);
    if (converter) {
        // Hacky fix for #2011, need to find a better way to choose the converter.
        if (converter === intersectionConverter &&
            !typeOrNode.isIntersection()) {
            converter = typeLiteralConverter;
        }
        const result = converter.convertType(context, typeOrNode, node);
        if (symbol)
            seenTypeSymbols.delete(symbol);
        return result;
    }
    return requestBugReport(context, typeOrNode);
}
exports.convertType = convertType;
const arrayConverter = {
    kind: [typescript_1.default.SyntaxKind.ArrayType],
    convert(context, node) {
        return new models_1.ArrayType(convertType(context, node.elementType));
    },
    convertType(context, type) {
        const params = context.checker.getTypeArguments(type);
        // This is *almost* always true... except for when this type is in the constraint of a type parameter see GH#1408
        // assert(params.length === 1);
        (0, assert_1.default)(params.length > 0);
        return new models_1.ArrayType(convertType(context, params[0]));
    },
};
const conditionalConverter = {
    kind: [typescript_1.default.SyntaxKind.ConditionalType],
    convert(context, node) {
        return new models_1.ConditionalType(convertType(context, node.checkType), convertType(context, node.extendsType), convertType(context, node.trueType), convertType(context, node.falseType));
    },
    convertType(context, type) {
        return new models_1.ConditionalType(convertType(context, type.checkType), convertType(context, type.extendsType), convertType(context, type.resolvedTrueType), convertType(context, type.resolvedFalseType));
    },
};
const constructorConverter = {
    kind: [typescript_1.default.SyntaxKind.ConstructorType],
    convert(context, node) {
        const symbol = context.getSymbolAtLocation(node) ?? node.symbol;
        const type = context.getTypeAtLocation(node);
        if (!symbol || !type) {
            return new models_1.IntrinsicType("Function");
        }
        const reflection = new models_1.DeclarationReflection("__type", models_1.ReflectionKind.Constructor, context.scope);
        const rc = context.withScope(reflection);
        rc.convertingTypeNode = true;
        context.registerReflection(reflection, symbol);
        context.trigger(converter_events_1.ConverterEvents.CREATE_DECLARATION, reflection);
        const signature = new models_1.SignatureReflection("__type", models_1.ReflectionKind.ConstructorSignature, reflection);
        // This is unfortunate... but seems the obvious place to put this with the current
        // architecture. Ideally, this would be a property on a "ConstructorType"... but that
        // needs to wait until TypeDoc 0.22 when making other breaking changes.
        if (node.modifiers?.some((m) => m.kind === typescript_1.default.SyntaxKind.AbstractKeyword)) {
            signature.setFlag(models_1.ReflectionFlag.Abstract);
        }
        context.project.registerSymbolId(signature, new ReflectionSymbolId_1.ReflectionSymbolId(symbol, node));
        context.registerReflection(signature, void 0);
        const signatureCtx = rc.withScope(signature);
        reflection.signatures = [signature];
        signature.type = convertType(signatureCtx, node.type);
        signature.parameters = (0, signature_1.convertParameterNodes)(signatureCtx, signature, node.parameters);
        signature.typeParameters = (0, signature_1.convertTypeParameterNodes)(signatureCtx, node.typeParameters);
        return new models_1.ReflectionType(reflection);
    },
    convertType(context, type) {
        if (!type.symbol) {
            return new models_1.IntrinsicType("Function");
        }
        const reflection = new models_1.DeclarationReflection("__type", models_1.ReflectionKind.Constructor, context.scope);
        context.registerReflection(reflection, type.symbol);
        context.trigger(converter_events_1.ConverterEvents.CREATE_DECLARATION, reflection);
        (0, signature_1.createSignature)(context.withScope(reflection), models_1.ReflectionKind.ConstructorSignature, type.getConstructSignatures()[0], type.symbol);
        return new models_1.ReflectionType(reflection);
    },
};
const exprWithTypeArgsConverter = {
    kind: [typescript_1.default.SyntaxKind.ExpressionWithTypeArguments],
    convert(context, node) {
        const targetSymbol = context.getSymbolAtLocation(node.expression);
        // Mixins... we might not have a symbol here.
        if (!targetSymbol) {
            return convertType(context, context.checker.getTypeAtLocation(node));
        }
        const parameters = node.typeArguments?.map((type) => convertType(context, type)) ?? [];
        const ref = models_1.ReferenceType.createSymbolReference(context.resolveAliasedSymbol(targetSymbol), context);
        ref.typeArguments = parameters;
        return ref;
    },
    convertType: requestBugReport,
};
const functionTypeConverter = {
    kind: [typescript_1.default.SyntaxKind.FunctionType],
    convert(context, node) {
        const symbol = context.getSymbolAtLocation(node) ?? node.symbol;
        const type = context.getTypeAtLocation(node);
        if (!symbol || !type) {
            return new models_1.IntrinsicType("Function");
        }
        const reflection = new models_1.DeclarationReflection("__type", models_1.ReflectionKind.TypeLiteral, context.scope);
        const rc = context.withScope(reflection);
        context.registerReflection(reflection, symbol);
        context.trigger(converter_events_1.ConverterEvents.CREATE_DECLARATION, reflection);
        const signature = new models_1.SignatureReflection("__type", models_1.ReflectionKind.CallSignature, reflection);
        context.project.registerSymbolId(signature, new ReflectionSymbolId_1.ReflectionSymbolId(symbol, node));
        context.registerReflection(signature, void 0);
        const signatureCtx = rc.withScope(signature);
        reflection.signatures = [signature];
        signature.type = convertType(signatureCtx, node.type);
        signature.parameters = (0, signature_1.convertParameterNodes)(signatureCtx, signature, node.parameters);
        signature.typeParameters = (0, signature_1.convertTypeParameterNodes)(signatureCtx, node.typeParameters);
        return new models_1.ReflectionType(reflection);
    },
    convertType(context, type) {
        if (!type.symbol) {
            return new models_1.IntrinsicType("Function");
        }
        const reflection = new models_1.DeclarationReflection("__type", models_1.ReflectionKind.TypeLiteral, context.scope);
        context.registerReflection(reflection, type.symbol);
        context.trigger(converter_events_1.ConverterEvents.CREATE_DECLARATION, reflection);
        (0, signature_1.createSignature)(context.withScope(reflection), models_1.ReflectionKind.CallSignature, type.getCallSignatures()[0], type.getSymbol());
        return new models_1.ReflectionType(reflection);
    },
};
const importType = {
    kind: [typescript_1.default.SyntaxKind.ImportType],
    convert(context, node) {
        const name = node.qualifier?.getText() ?? "__module";
        const symbol = context.checker.getSymbolAtLocation(node);
        (0, assert_1.default)(symbol, "Missing symbol when converting import type node");
        return models_1.ReferenceType.createSymbolReference(context.resolveAliasedSymbol(symbol), context, name);
    },
    convertType(context, type) {
        const symbol = type.getSymbol();
        (0, assert_1.default)(symbol, "Missing symbol when converting import type"); // Should be a compiler error
        return models_1.ReferenceType.createSymbolReference(context.resolveAliasedSymbol(symbol), context, "__module");
    },
};
const indexedAccessConverter = {
    kind: [typescript_1.default.SyntaxKind.IndexedAccessType],
    convert(context, node) {
        return new models_1.IndexedAccessType(convertType(context, node.objectType), convertType(context, node.indexType));
    },
    convertType(context, type) {
        return new models_1.IndexedAccessType(convertType(context, type.objectType), convertType(context, type.indexType));
    },
};
const inferredConverter = {
    kind: [typescript_1.default.SyntaxKind.InferType],
    convert(context, node) {
        return new models_1.InferredType(node.typeParameter.name.text, maybeConvertType(context, node.typeParameter.constraint));
    },
    convertType(context, type) {
        return new models_1.InferredType(type.symbol.name, maybeConvertType(context, type.getConstraint()));
    },
};
const intersectionConverter = {
    kind: [typescript_1.default.SyntaxKind.IntersectionType],
    convert(context, node) {
        return new models_1.IntersectionType(node.types.map((type) => convertType(context, type)));
    },
    convertType(context, type) {
        return new models_1.IntersectionType(type.types.map((type) => convertType(context, type)));
    },
};
const jsDocVariadicTypeConverter = {
    kind: [typescript_1.default.SyntaxKind.JSDocVariadicType],
    convert(context, node) {
        return new models_1.ArrayType(convertType(context, node.type));
    },
    // Should just be an ArrayType
    convertType: requestBugReport,
};
const keywordNames = {
    [typescript_1.default.SyntaxKind.AnyKeyword]: "any",
    [typescript_1.default.SyntaxKind.BigIntKeyword]: "bigint",
    [typescript_1.default.SyntaxKind.BooleanKeyword]: "boolean",
    [typescript_1.default.SyntaxKind.NeverKeyword]: "never",
    [typescript_1.default.SyntaxKind.NumberKeyword]: "number",
    [typescript_1.default.SyntaxKind.ObjectKeyword]: "object",
    [typescript_1.default.SyntaxKind.StringKeyword]: "string",
    [typescript_1.default.SyntaxKind.SymbolKeyword]: "symbol",
    [typescript_1.default.SyntaxKind.UndefinedKeyword]: "undefined",
    [typescript_1.default.SyntaxKind.UnknownKeyword]: "unknown",
    [typescript_1.default.SyntaxKind.VoidKeyword]: "void",
    [typescript_1.default.SyntaxKind.IntrinsicKeyword]: "intrinsic",
};
const keywordConverter = {
    kind: [
        typescript_1.default.SyntaxKind.AnyKeyword,
        typescript_1.default.SyntaxKind.BigIntKeyword,
        typescript_1.default.SyntaxKind.BooleanKeyword,
        typescript_1.default.SyntaxKind.NeverKeyword,
        typescript_1.default.SyntaxKind.NumberKeyword,
        typescript_1.default.SyntaxKind.ObjectKeyword,
        typescript_1.default.SyntaxKind.StringKeyword,
        typescript_1.default.SyntaxKind.SymbolKeyword,
        typescript_1.default.SyntaxKind.UndefinedKeyword,
        typescript_1.default.SyntaxKind.UnknownKeyword,
        typescript_1.default.SyntaxKind.VoidKeyword,
    ],
    convert(_context, node) {
        return new models_1.IntrinsicType(keywordNames[node.kind]);
    },
    convertType(_context, _type, node) {
        return new models_1.IntrinsicType(keywordNames[node.kind]);
    },
};
const optionalConverter = {
    kind: [typescript_1.default.SyntaxKind.OptionalType],
    convert(context, node) {
        return new models_1.OptionalType((0, reflections_1.removeUndefined)(convertType(context, node.type)));
    },
    // Handled by the tuple converter
    convertType: requestBugReport,
};
const parensConverter = {
    kind: [typescript_1.default.SyntaxKind.ParenthesizedType],
    convert(context, node) {
        return convertType(context, node.type);
    },
    // TS strips these out too... shouldn't run into this.
    convertType: requestBugReport,
};
const predicateConverter = {
    kind: [typescript_1.default.SyntaxKind.TypePredicate],
    convert(context, node) {
        const name = typescript_1.default.isThisTypeNode(node.parameterName)
            ? "this"
            : node.parameterName.getText();
        const asserts = !!node.assertsModifier;
        const targetType = node.type ? convertType(context, node.type) : void 0;
        return new models_1.PredicateType(name, asserts, targetType);
    },
    // Never inferred by TS 4.0, could potentially change in a future TS version.
    convertType: requestBugReport,
};
// This is a horrible thing... we're going to want to split this into converters
// for different types at some point.
const typeLiteralConverter = {
    kind: [typescript_1.default.SyntaxKind.TypeLiteral],
    convert(context, node) {
        const symbol = context.getSymbolAtLocation(node) ?? node.symbol;
        const type = context.getTypeAtLocation(node);
        if (!symbol || !type) {
            return new models_1.IntrinsicType("Object");
        }
        const reflection = new models_1.DeclarationReflection("__type", models_1.ReflectionKind.TypeLiteral, context.scope);
        const rc = context.withScope(reflection);
        rc.convertingTypeNode = true;
        context.registerReflection(reflection, symbol);
        context.trigger(converter_events_1.ConverterEvents.CREATE_DECLARATION, reflection);
        for (const prop of context.checker.getPropertiesOfType(type)) {
            (0, symbols_1.convertSymbol)(rc, prop);
        }
        for (const signature of type.getCallSignatures()) {
            (0, signature_1.createSignature)(rc, models_1.ReflectionKind.CallSignature, signature, symbol);
        }
        for (const signature of type.getConstructSignatures()) {
            (0, signature_1.createSignature)(rc, models_1.ReflectionKind.ConstructorSignature, signature, symbol);
        }
        (0, index_signature_1.convertIndexSignature)(rc, symbol);
        return new models_1.ReflectionType(reflection);
    },
    convertType(context, type) {
        if (!type.symbol) {
            return new models_1.IntrinsicType("Object");
        }
        const reflection = new models_1.DeclarationReflection("__type", models_1.ReflectionKind.TypeLiteral, context.scope);
        context.registerReflection(reflection, type.symbol);
        context.trigger(converter_events_1.ConverterEvents.CREATE_DECLARATION, reflection);
        for (const prop of context.checker.getPropertiesOfType(type)) {
            (0, symbols_1.convertSymbol)(context.withScope(reflection), prop);
        }
        for (const signature of type.getCallSignatures()) {
            (0, signature_1.createSignature)(context.withScope(reflection), models_1.ReflectionKind.CallSignature, signature, type.symbol);
        }
        for (const signature of type.getConstructSignatures()) {
            (0, signature_1.createSignature)(context.withScope(reflection), models_1.ReflectionKind.ConstructorSignature, signature, type.symbol);
        }
        (0, index_signature_1.convertIndexSignature)(context.withScope(reflection), type.symbol);
        return new models_1.ReflectionType(reflection);
    },
};
const queryConverter = {
    kind: [typescript_1.default.SyntaxKind.TypeQuery],
    convert(context, node) {
        const querySymbol = context.getSymbolAtLocation(node.exprName);
        if (!querySymbol) {
            // This can happen if someone uses `typeof` on some property
            // on a variable typed as `any` with a name that doesn't exist.
            return new models_1.QueryType(models_1.ReferenceType.createBrokenReference(node.exprName.getText(), context.project));
        }
        return new models_1.QueryType(models_1.ReferenceType.createSymbolReference(context.resolveAliasedSymbol(querySymbol), context, node.exprName.getText()));
    },
    convertType(context, type, node) {
        const symbol = type.getSymbol() || context.getSymbolAtLocation(node.exprName);
        (0, assert_1.default)(symbol, `Query type failed to get a symbol for: ${context.checker.typeToString(type)}. This is a bug.`);
        return new models_1.QueryType(models_1.ReferenceType.createSymbolReference(context.resolveAliasedSymbol(symbol), context));
    },
};
const referenceConverter = {
    kind: [typescript_1.default.SyntaxKind.TypeReference],
    convert(context, node) {
        const isArray = context.checker.typeToTypeNode(context.checker.getTypeAtLocation(node.typeName), void 0, typescript_1.default.NodeBuilderFlags.IgnoreErrors)?.kind === typescript_1.default.SyntaxKind.ArrayType;
        if (isArray) {
            return new models_1.ArrayType(convertType(context, node.typeArguments?.[0]));
        }
        const symbol = context.expectSymbolAtLocation(node.typeName);
        const name = node.typeName.getText();
        const type = models_1.ReferenceType.createSymbolReference(context.resolveAliasedSymbol(symbol), context, name);
        type.typeArguments = node.typeArguments?.map((type) => convertType(context, type));
        return type;
    },
    convertType(context, type) {
        const symbol = type.aliasSymbol ?? type.getSymbol();
        if (!symbol) {
            // This happens when we get a reference to a type parameter
            // created within a mapped type, `K` in: `{ [K in T]: string }`
            const ref = models_1.ReferenceType.createBrokenReference(context.checker.typeToString(type), context.project);
            ref.refersToTypeParameter = true;
            return ref;
        }
        const ref = models_1.ReferenceType.createSymbolReference(context.resolveAliasedSymbol(symbol), context);
        if (type.flags & typescript_1.default.TypeFlags.StringMapping) {
            ref.typeArguments = [
                convertType(context, type.type),
            ];
        }
        else {
            ref.typeArguments = (type.aliasSymbol
                ? type.aliasTypeArguments
                : type.typeArguments)?.map((ref) => convertType(context, ref));
        }
        return ref;
    },
};
const restConverter = {
    kind: [typescript_1.default.SyntaxKind.RestType],
    convert(context, node) {
        return new models_1.RestType(convertType(context, node.type));
    },
    // This is handled in the tuple converter
    convertType: requestBugReport,
};
const namedTupleMemberConverter = {
    kind: [typescript_1.default.SyntaxKind.NamedTupleMember],
    convert(context, node) {
        const innerType = convertType(context, node.type);
        return new models_1.NamedTupleMember(node.name.getText(), !!node.questionToken, innerType);
    },
    // This ought to be impossible.
    convertType: requestBugReport,
};
// { -readonly [K in string]-?: number}
//   ^ readonlyToken
//              ^ typeParameter
//                   ^^^^^^ typeParameter.constraint
//                          ^ questionToken
//                              ^^^^^^ type
const mappedConverter = {
    kind: [typescript_1.default.SyntaxKind.MappedType],
    convert(context, node) {
        const optionalModifier = kindToModifier(node.questionToken?.kind);
        const templateType = convertType(context, node.type);
        return new models_1.MappedType(node.typeParameter.name.text, convertType(context, node.typeParameter.constraint), optionalModifier === "+"
            ? (0, reflections_1.removeUndefined)(templateType)
            : templateType, kindToModifier(node.readonlyToken?.kind), optionalModifier, node.nameType ? convertType(context, node.nameType) : void 0);
    },
    convertType(context, type, node) {
        // This can happen if a generic function does not have a return type annotated.
        const optionalModifier = kindToModifier(node.questionToken?.kind);
        const templateType = convertType(context, type.templateType);
        return new models_1.MappedType(type.typeParameter.symbol?.name, convertType(context, type.typeParameter.getConstraint()), optionalModifier === "+"
            ? (0, reflections_1.removeUndefined)(templateType)
            : templateType, kindToModifier(node.readonlyToken?.kind), optionalModifier, type.nameType ? convertType(context, type.nameType) : void 0);
    },
};
const literalTypeConverter = {
    kind: [typescript_1.default.SyntaxKind.LiteralType],
    convert(context, node) {
        switch (node.literal.kind) {
            case typescript_1.default.SyntaxKind.TrueKeyword:
            case typescript_1.default.SyntaxKind.FalseKeyword:
                return new models_1.LiteralType(node.literal.kind === typescript_1.default.SyntaxKind.TrueKeyword);
            case typescript_1.default.SyntaxKind.StringLiteral:
                return new models_1.LiteralType(node.literal.text);
            case typescript_1.default.SyntaxKind.NumericLiteral:
                return new models_1.LiteralType(Number(node.literal.text));
            case typescript_1.default.SyntaxKind.NullKeyword:
                return new models_1.LiteralType(null);
            case typescript_1.default.SyntaxKind.PrefixUnaryExpression: {
                const operand = node.literal
                    .operand;
                switch (operand.kind) {
                    case typescript_1.default.SyntaxKind.NumericLiteral:
                        return new models_1.LiteralType(Number(node.literal.getText()));
                    case typescript_1.default.SyntaxKind.BigIntLiteral:
                        return new models_1.LiteralType(BigInt(node.literal.getText().replace("n", "")));
                    default:
                        return requestBugReport(context, node.literal);
                }
            }
            case typescript_1.default.SyntaxKind.BigIntLiteral:
                return new models_1.LiteralType(BigInt(node.literal.getText().replace("n", "")));
            case typescript_1.default.SyntaxKind.NoSubstitutionTemplateLiteral:
                return new models_1.LiteralType(node.literal.text);
        }
        return requestBugReport(context, node.literal);
    },
    convertType(_context, type, node) {
        switch (node.literal.kind) {
            case typescript_1.default.SyntaxKind.StringLiteral:
                return new models_1.LiteralType(node.literal.text);
            case typescript_1.default.SyntaxKind.NumericLiteral:
                return new models_1.LiteralType(+node.literal.text);
            case typescript_1.default.SyntaxKind.TrueKeyword:
            case typescript_1.default.SyntaxKind.FalseKeyword:
                return new models_1.LiteralType(node.literal.kind === typescript_1.default.SyntaxKind.TrueKeyword);
            case typescript_1.default.SyntaxKind.NullKeyword:
                return new models_1.LiteralType(null);
        }
        if (typeof type.value === "object") {
            return new models_1.LiteralType(BigInt(`${type.value.negative ? "-" : ""}${type.value.base10Value}`));
        }
        return new models_1.LiteralType(type.value);
    },
};
const templateLiteralConverter = {
    kind: [typescript_1.default.SyntaxKind.TemplateLiteralType],
    convert(context, node) {
        return new models_1.TemplateLiteralType(node.head.text, node.templateSpans.map((span) => {
            return [convertType(context, span.type), span.literal.text];
        }));
    },
    convertType(context, type) {
        (0, assert_1.default)(type.texts.length === type.types.length + 1);
        const parts = [];
        for (const [a, b] of (0, array_1.zip)(type.types, type.texts.slice(1))) {
            parts.push([convertType(context, a), b]);
        }
        return new models_1.TemplateLiteralType(type.texts[0], parts);
    },
};
const thisConverter = {
    kind: [typescript_1.default.SyntaxKind.ThisType],
    convert() {
        return new models_1.IntrinsicType("this");
    },
    convertType() {
        return new models_1.IntrinsicType("this");
    },
};
const tupleConverter = {
    kind: [typescript_1.default.SyntaxKind.TupleType],
    convert(context, node) {
        const elements = node.elements.map((node) => convertType(context, node));
        return new models_1.TupleType(elements);
    },
    convertType(context, type, node) {
        const types = type.typeArguments?.slice(0, node.elements.length);
        let elements = types?.map((type) => convertType(context, type));
        if (type.target.labeledElementDeclarations) {
            const namedDeclarations = type.target.labeledElementDeclarations;
            elements = elements?.map((el, i) => new models_1.NamedTupleMember(namedDeclarations[i].name.getText(), !!namedDeclarations[i].questionToken, (0, reflections_1.removeUndefined)(el)));
        }
        elements = elements?.map((el, i) => {
            if (type.target.elementFlags[i] & typescript_1.default.ElementFlags.Variable) {
                // In the node case, we don't need to add the wrapping Array type... but we do here.
                if (el instanceof models_1.NamedTupleMember) {
                    return new models_1.RestType(new models_1.NamedTupleMember(el.name, el.isOptional, new models_1.ArrayType(el.element)));
                }
                return new models_1.RestType(new models_1.ArrayType(el));
            }
            if (type.target.elementFlags[i] & typescript_1.default.ElementFlags.Optional &&
                !(el instanceof models_1.NamedTupleMember)) {
                return new models_1.OptionalType((0, reflections_1.removeUndefined)(el));
            }
            return el;
        });
        return new models_1.TupleType(elements ?? []);
    },
};
const supportedOperatorNames = {
    [typescript_1.default.SyntaxKind.KeyOfKeyword]: "keyof",
    [typescript_1.default.SyntaxKind.UniqueKeyword]: "unique",
    [typescript_1.default.SyntaxKind.ReadonlyKeyword]: "readonly",
};
const typeOperatorConverter = {
    kind: [typescript_1.default.SyntaxKind.TypeOperator],
    convert(context, node) {
        return new models_1.TypeOperatorType(convertType(context, node.type), supportedOperatorNames[node.operator]);
    },
    convertType(context, type, node) {
        // readonly is only valid on array and tuple literal types.
        if (node.operator === typescript_1.default.SyntaxKind.ReadonlyKeyword) {
            const resolved = resolveReference(type);
            (0, assert_1.default)((0, nodes_1.isObjectType)(resolved));
            const args = context.checker
                .getTypeArguments(type)
                .map((type) => convertType(context, type));
            const inner = resolved.objectFlags & typescript_1.default.ObjectFlags.Tuple
                ? new models_1.TupleType(args)
                : new models_1.ArrayType(args[0]);
            return new models_1.TypeOperatorType(inner, "readonly");
        }
        // keyof will only show up with generic functions, otherwise it gets eagerly
        // resolved to a union of strings.
        if (node.operator === typescript_1.default.SyntaxKind.KeyOfKeyword) {
            // TS 4.2 added this to enable better tracking of type aliases.
            if (type.isUnion() && type.origin) {
                return convertType(context, type.origin);
            }
            // There's probably an interface for this somewhere... I couldn't find it.
            const targetType = type.type;
            return new models_1.TypeOperatorType(convertType(context, targetType), "keyof");
        }
        // TS drops `unique` in `unique symbol` everywhere. If someone used it, we ought
        // to have a type node. This shouldn't ever happen.
        return requestBugReport(context, type);
    },
};
const unionConverter = {
    kind: [typescript_1.default.SyntaxKind.UnionType],
    convert(context, node) {
        return new models_1.UnionType(node.types.map((type) => convertType(context, type)));
    },
    convertType(context, type) {
        // TS 4.2 added this to enable better tracking of type aliases.
        if (type.origin) {
            return convertType(context, type.origin);
        }
        return new models_1.UnionType(type.types.map((type) => convertType(context, type)));
    },
};
const jsDocNullableTypeConverter = {
    kind: [typescript_1.default.SyntaxKind.JSDocNullableType],
    convert(context, node) {
        return new models_1.UnionType([
            convertType(context, node.type),
            new models_1.LiteralType(null),
        ]);
    },
    // Should be a UnionType
    convertType: requestBugReport,
};
const jsDocNonNullableTypeConverter = {
    kind: [typescript_1.default.SyntaxKind.JSDocNonNullableType],
    convert(context, node) {
        return convertType(context, node.type);
    },
    // Should be a UnionType
    convertType: requestBugReport,
};
function requestBugReport(context, nodeOrType) {
    if ("kind" in nodeOrType) {
        const kindName = typescript_1.default.SyntaxKind[nodeOrType.kind];
        context.logger.warn(`Failed to convert type node with kind: ${kindName} and text ${nodeOrType.getText()}. Please report a bug.`, nodeOrType);
        return new models_1.UnknownType(nodeOrType.getText());
    }
    else {
        const typeString = context.checker.typeToString(nodeOrType);
        context.logger.warn(`Failed to convert type: ${typeString} when converting ${context.scope.getFullName()}. Please report a bug.`);
        return new models_1.UnknownType(typeString);
    }
}
function resolveReference(type) {
    if ((0, nodes_1.isObjectType)(type) && type.objectFlags & typescript_1.default.ObjectFlags.Reference) {
        return type.target;
    }
    return type;
}
function kindToModifier(kind) {
    switch (kind) {
        case typescript_1.default.SyntaxKind.ReadonlyKeyword:
        case typescript_1.default.SyntaxKind.QuestionToken:
        case typescript_1.default.SyntaxKind.PlusToken:
            return "+";
        case typescript_1.default.SyntaxKind.MinusToken:
            return "-";
        default:
            return undefined;
    }
}
