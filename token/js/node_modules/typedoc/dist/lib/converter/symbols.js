"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.convertSymbol = void 0;
const assert_1 = __importDefault(require("assert"));
const typescript_1 = __importDefault(require("typescript"));
const models_1 = require("../models");
const enum_1 = require("../utils/enum");
const convert_expression_1 = require("./convert-expression");
const index_signature_1 = require("./factories/index-signature");
const signature_1 = require("./factories/signature");
const jsdoc_1 = require("./jsdoc");
const nodes_1 = require("./utils/nodes");
const reflections_1 = require("./utils/reflections");
const symbolConverters = {
    [typescript_1.default.SymbolFlags.RegularEnum]: convertEnum,
    [typescript_1.default.SymbolFlags.ConstEnum]: convertEnum,
    [typescript_1.default.SymbolFlags.EnumMember]: convertEnumMember,
    [typescript_1.default.SymbolFlags.ValueModule]: convertNamespace,
    [typescript_1.default.SymbolFlags.NamespaceModule]: convertNamespace,
    [typescript_1.default.SymbolFlags.TypeAlias]: convertTypeAlias,
    [typescript_1.default.SymbolFlags.Function]: convertFunctionOrMethod,
    [typescript_1.default.SymbolFlags.Method]: convertFunctionOrMethod,
    [typescript_1.default.SymbolFlags.Interface]: convertClassOrInterface,
    [typescript_1.default.SymbolFlags.Property]: convertProperty,
    [typescript_1.default.SymbolFlags.Class]: convertClassOrInterface,
    [typescript_1.default.SymbolFlags.Constructor]: convertConstructor,
    [typescript_1.default.SymbolFlags.Alias]: convertAlias,
    [typescript_1.default.SymbolFlags.BlockScopedVariable]: convertVariable,
    [typescript_1.default.SymbolFlags.FunctionScopedVariable]: convertVariable,
    [typescript_1.default.SymbolFlags.ExportValue]: convertVariable,
    [typescript_1.default.SymbolFlags.GetAccessor]: convertAccessor,
    [typescript_1.default.SymbolFlags.SetAccessor]: convertAccessor,
};
const allConverterFlags = Object.keys(symbolConverters).reduce((v, k) => v | +k, 0);
// This is kind of a hack, born of resolving references by symbols instead
// of by source location.
const conversionOrder = [
    // Do enums before namespaces so that @hidden on a namespace
    // merged with an enum works properly.
    typescript_1.default.SymbolFlags.RegularEnum,
    typescript_1.default.SymbolFlags.ConstEnum,
    typescript_1.default.SymbolFlags.EnumMember,
    // Before type alias
    typescript_1.default.SymbolFlags.BlockScopedVariable,
    typescript_1.default.SymbolFlags.FunctionScopedVariable,
    typescript_1.default.SymbolFlags.ExportValue,
    typescript_1.default.SymbolFlags.TypeAlias,
    typescript_1.default.SymbolFlags.Function,
    typescript_1.default.SymbolFlags.Method,
    typescript_1.default.SymbolFlags.Interface,
    typescript_1.default.SymbolFlags.Property,
    typescript_1.default.SymbolFlags.Class,
    typescript_1.default.SymbolFlags.Constructor,
    typescript_1.default.SymbolFlags.Alias,
    typescript_1.default.SymbolFlags.GetAccessor,
    typescript_1.default.SymbolFlags.SetAccessor,
    typescript_1.default.SymbolFlags.ValueModule,
    typescript_1.default.SymbolFlags.NamespaceModule,
];
// Sanity check, if this fails a dev messed up.
for (const key of Object.keys(symbolConverters)) {
    if (!Number.isInteger(Math.log2(+key))) {
        throw new Error(`Symbol converter for key ${typescript_1.default.SymbolFlags[+key]} does not specify a valid flag value.`);
    }
    if (!conversionOrder.includes(+key)) {
        throw new Error(`Symbol converter for key ${typescript_1.default.SymbolFlags[+key]} is not specified in conversionOrder`);
    }
}
if (conversionOrder.reduce((a, b) => a | b, 0) !== allConverterFlags) {
    throw new Error("conversionOrder contains a symbol flag that converters do not.");
}
function convertSymbol(context, symbol, exportSymbol) {
    if (context.shouldIgnore(symbol)) {
        return;
    }
    // This check can catch symbols which ought to be documented as references
    // but aren't aliased symbols because `export *` was used.
    const previous = context.project.getReflectionFromSymbol(symbol);
    if (previous &&
        previous.parent?.kindOf(models_1.ReflectionKind.SomeModule | models_1.ReflectionKind.Project)) {
        createAlias(previous, context, symbol, exportSymbol);
        return;
    }
    let flags = (0, enum_1.removeFlag)(symbol.flags, typescript_1.default.SymbolFlags.Transient |
        typescript_1.default.SymbolFlags.Assignment |
        typescript_1.default.SymbolFlags.Optional |
        typescript_1.default.SymbolFlags.Prototype);
    // Declaration merging - the only type (excluding enum/enum, ns/ns, etc)
    // that TD supports is merging a class and interface. All others are
    // represented as multiple reflections
    if ((0, enum_1.hasAllFlags)(symbol.flags, typescript_1.default.SymbolFlags.Class)) {
        flags = (0, enum_1.removeFlag)(flags, typescript_1.default.SymbolFlags.Interface | typescript_1.default.SymbolFlags.Function);
    }
    // Kind of declaration merging... we treat this as a property with get/set signatures.
    if ((0, enum_1.hasAllFlags)(symbol.flags, typescript_1.default.SymbolFlags.GetAccessor)) {
        flags = (0, enum_1.removeFlag)(flags, typescript_1.default.SymbolFlags.SetAccessor);
    }
    if ((0, enum_1.hasAllFlags)(symbol.flags, typescript_1.default.SymbolFlags.NamespaceModule)) {
        // This might be here if a namespace is declared several times.
        flags = (0, enum_1.removeFlag)(flags, typescript_1.default.SymbolFlags.ValueModule);
    }
    if ((0, enum_1.hasAnyFlag)(symbol.flags, typescript_1.default.SymbolFlags.Method |
        typescript_1.default.SymbolFlags.Interface |
        typescript_1.default.SymbolFlags.Class |
        typescript_1.default.SymbolFlags.Variable)) {
        // This happens when someone declares an object with methods:
        // { methodProperty() {} }
        flags = (0, enum_1.removeFlag)(flags, typescript_1.default.SymbolFlags.Property);
    }
    // A default exported function with no associated variable is a property, but
    // we should really convert it as a variable for documentation purposes
    // export default () => {}
    // export default 123
    if (flags === typescript_1.default.SymbolFlags.Property &&
        symbol.name === "default" &&
        context.scope.kindOf(models_1.ReflectionKind.Module | models_1.ReflectionKind.Project)) {
        flags = typescript_1.default.SymbolFlags.BlockScopedVariable;
    }
    for (const flag of (0, enum_1.getEnumFlags)(flags ^ allConverterFlags)) {
        if (!(flag & allConverterFlags)) {
            context.logger.verbose(`Missing converter for symbol: ${symbol.name} with flag ${typescript_1.default.SymbolFlags[flag]}`);
        }
    }
    // Note: This method does not allow skipping earlier converters.
    // For now, this is fine... might not be flexible enough in the future.
    let skip = 0;
    for (const flag of conversionOrder) {
        if (!(flag & flags))
            continue;
        if (skip & flag)
            continue;
        skip |= symbolConverters[flag]?.(context, symbol, exportSymbol) || 0;
    }
}
exports.convertSymbol = convertSymbol;
function convertSymbols(context, symbols) {
    for (const symbol of symbols) {
        convertSymbol(context, symbol);
    }
}
function convertEnum(context, symbol, exportSymbol) {
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Enum, symbol, exportSymbol);
    if (symbol.flags & typescript_1.default.SymbolFlags.ConstEnum) {
        reflection.setFlag(models_1.ReflectionFlag.Const);
    }
    context.finalizeDeclarationReflection(reflection);
    convertSymbols(context.withScope(reflection), context.checker
        .getExportsOfModule(symbol)
        .filter((s) => s.flags & typescript_1.default.SymbolFlags.EnumMember));
}
function convertEnumMember(context, symbol, exportSymbol) {
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.EnumMember, symbol, exportSymbol);
    const defaultValue = context.checker.getConstantValue(symbol.getDeclarations()[0]);
    if (defaultValue !== undefined) {
        reflection.type = new models_1.LiteralType(defaultValue);
    }
    else {
        // We know this has to be a number, because computed values aren't allowed
        // in string enums, so otherwise we would have to have the constant value
        reflection.type = new models_1.IntrinsicType("number");
    }
    context.finalizeDeclarationReflection(reflection);
}
function convertNamespace(context, symbol, exportSymbol) {
    let exportFlags = typescript_1.default.SymbolFlags.ModuleMember;
    // This can happen in JS land where "class" functions get tagged as a namespace too
    if (symbol
        .getDeclarations()
        ?.some((d) => typescript_1.default.isModuleDeclaration(d) || typescript_1.default.isSourceFile(d)) !==
        true) {
        exportFlags = typescript_1.default.SymbolFlags.ClassMember;
        if ((0, enum_1.hasAnyFlag)(symbol.flags, typescript_1.default.SymbolFlags.Class)) {
            return;
        }
    }
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Namespace, symbol, exportSymbol);
    context.finalizeDeclarationReflection(reflection);
    convertSymbols(context.withScope(reflection), context.checker
        .getExportsOfModule(symbol)
        .filter((s) => s.flags & exportFlags));
}
function convertTypeAlias(context, symbol, exportSymbol) {
    const declaration = symbol
        ?.getDeclarations()
        ?.find((d) => typescript_1.default.isTypeAliasDeclaration(d) ||
        typescript_1.default.isJSDocTypedefTag(d) ||
        typescript_1.default.isJSDocCallbackTag(d) ||
        typescript_1.default.isJSDocEnumTag(d));
    (0, assert_1.default)(declaration);
    if (typescript_1.default.isTypeAliasDeclaration(declaration)) {
        if (context
            .getComment(symbol, models_1.ReflectionKind.TypeAlias)
            ?.hasModifier("@interface")) {
            return convertTypeAliasAsInterface(context, symbol, exportSymbol, declaration);
        }
        const reflection = context.createDeclarationReflection(models_1.ReflectionKind.TypeAlias, symbol, exportSymbol);
        reflection.type = context.converter.convertType(context.withScope(reflection), declaration.type);
        context.finalizeDeclarationReflection(reflection);
        // Do this after finalization so that the CommentPlugin can get @typeParam tags
        // from the parent comment. Ugly, but works for now. Should be cleaned up eventually.
        reflection.typeParameters = declaration.typeParameters?.map((param) => (0, signature_1.createTypeParamReflection)(param, context.withScope(reflection)));
    }
    else if (typescript_1.default.isJSDocTypedefTag(declaration) ||
        typescript_1.default.isJSDocEnumTag(declaration)) {
        (0, jsdoc_1.convertJsDocAlias)(context, symbol, declaration, exportSymbol);
    }
    else {
        (0, jsdoc_1.convertJsDocCallback)(context, symbol, declaration, exportSymbol);
    }
}
function convertTypeAliasAsInterface(context, symbol, exportSymbol, declaration) {
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Interface, symbol, exportSymbol);
    context.finalizeDeclarationReflection(reflection);
    const rc = context.withScope(reflection);
    const type = context.checker.getTypeAtLocation(declaration);
    // Interfaces have properties
    convertSymbols(rc, type.getProperties());
    // And type arguments
    if (declaration.typeParameters) {
        reflection.typeParameters = declaration.typeParameters.map((param) => {
            const declaration = param.symbol?.declarations?.[0];
            (0, assert_1.default)(declaration && typescript_1.default.isTypeParameterDeclaration(declaration));
            return (0, signature_1.createTypeParamReflection)(declaration, rc);
        });
    }
    // And maybe call signatures
    context.checker
        .getSignaturesOfType(type, typescript_1.default.SignatureKind.Call)
        .forEach((sig) => (0, signature_1.createSignature)(rc, models_1.ReflectionKind.CallSignature, sig, symbol));
    // And maybe constructor signatures
    convertConstructSignatures(rc, symbol);
    // And finally, index signatures
    (0, index_signature_1.convertIndexSignature)(rc, symbol);
}
function convertFunctionOrMethod(context, symbol, exportSymbol) {
    // Can't just check method flag because this might be called for properties as well
    // This will *NOT* be called for variables that look like functions, they need a special case.
    const isMethod = !!(symbol.flags &
        (typescript_1.default.SymbolFlags.Property | typescript_1.default.SymbolFlags.Method));
    const declarations = symbol.getDeclarations()?.filter(typescript_1.default.isFunctionLike) ?? [];
    // Don't do anything if we inherited this method and it is private.
    if (isMethod &&
        isInherited(context, symbol) &&
        declarations.length > 0 &&
        (0, enum_1.hasAllFlags)(typescript_1.default.getCombinedModifierFlags(declarations[0]), typescript_1.default.ModifierFlags.Private)) {
        return;
    }
    const locationDeclaration = symbol.parent
        ?.getDeclarations()
        ?.find((d) => typescript_1.default.isClassDeclaration(d) || typescript_1.default.isInterfaceDeclaration(d)) ??
        symbol.parent?.getDeclarations()?.[0]?.getSourceFile() ??
        symbol.getDeclarations()?.[0]?.getSourceFile();
    (0, assert_1.default)(locationDeclaration, "Missing declaration context");
    const type = context.checker.getTypeOfSymbolAtLocation(symbol, locationDeclaration);
    // Need to get the non nullable type because interface methods might be declared
    // with a question token. See GH1490.
    const signatures = type.getNonNullableType().getCallSignatures();
    const reflection = context.createDeclarationReflection(context.scope.kindOf(models_1.ReflectionKind.ClassOrInterface |
        models_1.ReflectionKind.VariableOrProperty |
        models_1.ReflectionKind.TypeLiteral)
        ? models_1.ReflectionKind.Method
        : models_1.ReflectionKind.Function, symbol, exportSymbol, void 0);
    if (symbol.declarations?.length && isMethod) {
        // All method signatures must have the same modifier flags.
        setModifiers(symbol, symbol.declarations[0], reflection);
    }
    context.finalizeDeclarationReflection(reflection);
    const scope = context.withScope(reflection);
    // Can't use zip here. We might have less declarations than signatures
    // or less signatures than declarations.
    for (const sig of signatures) {
        (0, signature_1.createSignature)(scope, models_1.ReflectionKind.CallSignature, sig, symbol);
    }
}
// getDeclaredTypeOfSymbol gets the INSTANCE type
// getTypeOfSymbolAtLocation gets the STATIC type
function convertClassOrInterface(context, symbol, exportSymbol) {
    const reflection = context.createDeclarationReflection(typescript_1.default.SymbolFlags.Class & symbol.flags
        ? models_1.ReflectionKind.Class
        : models_1.ReflectionKind.Interface, symbol, exportSymbol, void 0);
    const classDeclaration = symbol
        .getDeclarations()
        ?.find((d) => typescript_1.default.isClassDeclaration(d) || typescript_1.default.isFunctionDeclaration(d));
    if (classDeclaration)
        setModifiers(symbol, classDeclaration, reflection);
    const reflectionContext = context.withScope(reflection);
    reflectionContext.convertingClassOrInterface = true;
    const instanceType = context.checker.getDeclaredTypeOfSymbol(symbol);
    (0, assert_1.default)(instanceType.isClassOrInterface());
    // We might do some inheritance - do this first so that it's set when converting properties
    const declarations = symbol
        .getDeclarations()
        ?.filter((d) => typescript_1.default.isInterfaceDeclaration(d) || typescript_1.default.isClassDeclaration(d)) ?? [];
    const extendedTypes = (0, nodes_1.getHeritageTypes)(declarations, typescript_1.default.SyntaxKind.ExtendsKeyword).map((t) => context.converter.convertType(reflectionContext, t));
    if (extendedTypes.length) {
        reflection.extendedTypes = extendedTypes;
    }
    const implementedTypes = (0, nodes_1.getHeritageTypes)(declarations, typescript_1.default.SyntaxKind.ImplementsKeyword).map((t) => context.converter.convertType(reflectionContext, t));
    if (implementedTypes.length) {
        reflection.implementedTypes = implementedTypes;
    }
    context.finalizeDeclarationReflection(reflection);
    if (classDeclaration) {
        // Classes can have static props
        const staticType = context.checker.getTypeOfSymbolAtLocation(symbol, classDeclaration);
        reflectionContext.shouldBeStatic = true;
        for (const prop of context.checker.getPropertiesOfType(staticType)) {
            // Don't convert namespace members, or the prototype here.
            if (prop.flags &
                (typescript_1.default.SymbolFlags.ModuleMember | typescript_1.default.SymbolFlags.Prototype))
                continue;
            convertSymbol(reflectionContext, prop);
        }
        reflectionContext.shouldBeStatic = false;
        const ctors = staticType.getConstructSignatures();
        const constructMember = reflectionContext.createDeclarationReflection(models_1.ReflectionKind.Constructor, ctors?.[0]?.declaration?.symbol, void 0, "constructor");
        // Modifiers are the same for all constructors
        if (ctors.length && ctors[0].declaration) {
            setModifiers(symbol, ctors[0].declaration, constructMember);
        }
        context.finalizeDeclarationReflection(constructMember);
        const constructContext = reflectionContext.withScope(constructMember);
        ctors.forEach((sig) => {
            (0, signature_1.createSignature)(constructContext, models_1.ReflectionKind.ConstructorSignature, sig, symbol);
        });
    }
    // Classes/interfaces usually just have properties...
    convertSymbols(reflectionContext, context.checker.getPropertiesOfType(instanceType));
    // And type arguments
    if (instanceType.typeParameters) {
        reflection.typeParameters = instanceType.typeParameters.map((param) => {
            const declaration = param.symbol?.declarations?.[0];
            (0, assert_1.default)(declaration && typescript_1.default.isTypeParameterDeclaration(declaration));
            return (0, signature_1.createTypeParamReflection)(declaration, reflectionContext);
        });
    }
    // Interfaces might also have call signatures
    // Classes might too, because of declaration merging
    context.checker
        .getSignaturesOfType(instanceType, typescript_1.default.SignatureKind.Call)
        .forEach((sig) => (0, signature_1.createSignature)(reflectionContext, models_1.ReflectionKind.CallSignature, sig, symbol));
    // We also might have constructor signatures
    // This is potentially a problem with classes having multiple "constructor" members...
    // but nobody has complained yet.
    convertConstructSignatures(reflectionContext, symbol);
    // And finally, index signatures
    (0, index_signature_1.convertIndexSignature)(reflectionContext, symbol);
}
function convertProperty(context, symbol, exportSymbol) {
    const declarations = symbol.getDeclarations() ?? [];
    // Don't do anything if we inherited this property and it is private.
    if (isInherited(context, symbol) &&
        declarations.length > 0 &&
        (0, enum_1.hasAllFlags)(typescript_1.default.getCombinedModifierFlags(declarations[0]), typescript_1.default.ModifierFlags.Private)) {
        return;
    }
    // Special case: We pretend properties are methods if they look like methods.
    // This happens with mixins / weird inheritance.
    if (declarations.length &&
        declarations.every((decl) => typescript_1.default.isMethodSignature(decl) || typescript_1.default.isMethodDeclaration(decl))) {
        return convertFunctionOrMethod(context, symbol, exportSymbol);
    }
    if (declarations.length === 1) {
        const declaration = declarations[0];
        // Special case: "arrow methods" should be treated as methods.
        if (typescript_1.default.isPropertyDeclaration(declaration) &&
            !declaration.type &&
            declaration.initializer &&
            typescript_1.default.isArrowFunction(declaration.initializer)) {
            return convertArrowAsMethod(context, symbol, declaration.initializer, exportSymbol);
        }
    }
    const reflection = context.createDeclarationReflection(context.scope.kindOf(models_1.ReflectionKind.Namespace)
        ? models_1.ReflectionKind.Variable
        : models_1.ReflectionKind.Property, symbol, exportSymbol);
    reflection.conversionFlags |= models_1.ConversionFlags.VariableOrPropertySource;
    const declaration = symbol.getDeclarations()?.[0];
    let parameterType;
    if (declaration &&
        (typescript_1.default.isPropertyDeclaration(declaration) ||
            typescript_1.default.isPropertySignature(declaration) ||
            typescript_1.default.isParameter(declaration) ||
            typescript_1.default.isPropertyAccessExpression(declaration) ||
            typescript_1.default.isPropertyAssignment(declaration))) {
        if (!typescript_1.default.isPropertyAccessExpression(declaration) &&
            !typescript_1.default.isPropertyAssignment(declaration)) {
            parameterType = declaration.type;
        }
        setModifiers(symbol, declaration, reflection);
    }
    reflection.defaultValue = declaration && (0, convert_expression_1.convertDefaultValue)(declaration);
    reflection.type = context.converter.convertType(context.withScope(reflection), (context.convertingTypeNode ? parameterType : void 0) ??
        context.checker.getTypeOfSymbol(symbol));
    if (reflection.flags.isOptional) {
        reflection.type = (0, reflections_1.removeUndefined)(reflection.type);
    }
    context.finalizeDeclarationReflection(reflection);
}
function convertArrowAsMethod(context, symbol, arrow, exportSymbol) {
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Method, symbol, exportSymbol, void 0);
    setModifiers(symbol, arrow.parent, reflection);
    context.finalizeDeclarationReflection(reflection);
    const rc = context.withScope(reflection);
    const signature = context.checker.getSignatureFromDeclaration(arrow);
    (0, assert_1.default)(signature);
    (0, signature_1.createSignature)(rc, models_1.ReflectionKind.CallSignature, signature, symbol, arrow);
}
function convertConstructor(context, symbol) {
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Constructor, symbol, void 0, "constructor");
    context.finalizeDeclarationReflection(reflection);
    const reflectionContext = context.withScope(reflection);
    const declarations = symbol.getDeclarations()?.filter(typescript_1.default.isConstructorDeclaration) ?? [];
    const signatures = declarations.map((decl) => {
        const sig = context.checker.getSignatureFromDeclaration(decl);
        (0, assert_1.default)(sig);
        return sig;
    });
    for (const sig of signatures) {
        (0, signature_1.createSignature)(reflectionContext, models_1.ReflectionKind.ConstructorSignature, sig, symbol);
    }
}
function convertConstructSignatures(context, symbol) {
    const type = context.checker.getDeclaredTypeOfSymbol(symbol);
    // These get added as a "constructor" member of this interface. This is a problem... but nobody
    // has complained yet. We really ought to have a constructSignatures property on the reflection instead.
    const constructSignatures = context.checker.getSignaturesOfType(type, typescript_1.default.SignatureKind.Construct);
    if (constructSignatures.length) {
        const constructMember = new models_1.DeclarationReflection("constructor", models_1.ReflectionKind.Constructor, context.scope);
        context.postReflectionCreation(constructMember, symbol, void 0);
        context.finalizeDeclarationReflection(constructMember);
        const constructContext = context.withScope(constructMember);
        constructSignatures.forEach((sig) => (0, signature_1.createSignature)(constructContext, models_1.ReflectionKind.ConstructorSignature, sig, symbol));
    }
}
function convertAlias(context, symbol, exportSymbol) {
    const reflection = context.project.getReflectionFromSymbol(context.resolveAliasedSymbol(symbol));
    if (!reflection) {
        // We don't have this, convert it.
        convertSymbol(context, context.resolveAliasedSymbol(symbol), exportSymbol ?? symbol);
    }
    else {
        createAlias(reflection, context, symbol, exportSymbol);
    }
}
function createAlias(target, context, symbol, exportSymbol) {
    if (context.converter.excludeReferences)
        return;
    // We already have this. Create a reference.
    const ref = new models_1.ReferenceReflection(exportSymbol?.name ?? symbol.name, target, context.scope);
    context.postReflectionCreation(ref, symbol, exportSymbol);
    context.finalizeDeclarationReflection(ref);
}
function convertVariable(context, symbol, exportSymbol) {
    const declaration = symbol.getDeclarations()?.[0];
    (0, assert_1.default)(declaration);
    const comment = context.getComment(symbol, models_1.ReflectionKind.Variable);
    const type = context.checker.getTypeOfSymbolAtLocation(symbol, declaration);
    if (isEnumLike(context.checker, type, declaration) &&
        comment?.hasModifier("@enum")) {
        return convertVariableAsEnum(context, symbol, exportSymbol);
    }
    if (comment?.hasModifier("@namespace")) {
        return convertVariableAsNamespace(context, symbol, exportSymbol);
    }
    if (type.getCallSignatures().length) {
        return convertVariableAsFunction(context, symbol, exportSymbol);
    }
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Variable, symbol, exportSymbol);
    let typeNode;
    if (typescript_1.default.isVariableDeclaration(declaration)) {
        // Otherwise we might have destructuring
        typeNode = declaration.type;
    }
    reflection.type = context.converter.convertType(context.withScope(reflection), typeNode ?? type);
    setModifiers(symbol, declaration, reflection);
    reflection.defaultValue = (0, convert_expression_1.convertDefaultValue)(declaration);
    context.finalizeDeclarationReflection(reflection);
    return typescript_1.default.SymbolFlags.Property;
}
function isEnumLike(checker, type, location) {
    if (!(0, enum_1.hasAllFlags)(type.flags, typescript_1.default.TypeFlags.Object)) {
        return false;
    }
    return type.getProperties().every((prop) => {
        const propType = checker.getTypeOfSymbolAtLocation(prop, location);
        return isValidEnumProperty(propType);
    });
}
function isValidEnumProperty(type) {
    return (0, enum_1.hasAnyFlag)(type.flags, typescript_1.default.TypeFlags.NumberLike | typescript_1.default.TypeFlags.StringLike);
}
function convertVariableAsEnum(context, symbol, exportSymbol) {
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Enum, symbol, exportSymbol);
    context.finalizeDeclarationReflection(reflection);
    const rc = context.withScope(reflection);
    const declaration = symbol.declarations[0];
    const type = context.checker.getTypeAtLocation(declaration);
    for (const prop of type.getProperties()) {
        const reflection = rc.createDeclarationReflection(models_1.ReflectionKind.EnumMember, prop, void 0);
        const propType = context.checker.getTypeOfSymbolAtLocation(prop, declaration);
        reflection.type = context.converter.convertType(context, propType);
        rc.finalizeDeclarationReflection(reflection);
    }
    // Skip converting the type alias, if there is one
    return typescript_1.default.SymbolFlags.TypeAlias;
}
function convertVariableAsNamespace(context, symbol, exportSymbol) {
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Namespace, symbol, exportSymbol);
    context.finalizeDeclarationReflection(reflection);
    const rc = context.withScope(reflection);
    const declaration = symbol.declarations[0];
    const type = context.checker.getTypeAtLocation(declaration);
    convertSymbols(rc, type.getProperties());
    return typescript_1.default.SymbolFlags.Property;
}
function convertVariableAsFunction(context, symbol, exportSymbol) {
    const declaration = symbol
        .getDeclarations()
        ?.find(typescript_1.default.isVariableDeclaration);
    const accessDeclaration = declaration ?? symbol.valueDeclaration;
    const type = accessDeclaration
        ? context.checker.getTypeOfSymbolAtLocation(symbol, accessDeclaration)
        : context.checker.getDeclaredTypeOfSymbol(symbol);
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Function, symbol, exportSymbol);
    setModifiers(symbol, accessDeclaration, reflection);
    reflection.conversionFlags |= models_1.ConversionFlags.VariableOrPropertySource;
    context.finalizeDeclarationReflection(reflection);
    const reflectionContext = context.withScope(reflection);
    reflection.signatures ?? (reflection.signatures = []);
    for (const signature of type.getCallSignatures()) {
        (0, signature_1.createSignature)(reflectionContext, models_1.ReflectionKind.CallSignature, signature, symbol);
    }
    return typescript_1.default.SymbolFlags.Property;
}
function convertAccessor(context, symbol, exportSymbol) {
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Accessor, symbol, exportSymbol);
    const rc = context.withScope(reflection);
    const declaration = symbol.getDeclarations()?.[0];
    if (declaration) {
        setModifiers(symbol, declaration, reflection);
    }
    context.finalizeDeclarationReflection(reflection);
    const getDeclaration = symbol.getDeclarations()?.find(typescript_1.default.isGetAccessor);
    if (getDeclaration) {
        const signature = context.checker.getSignatureFromDeclaration(getDeclaration);
        if (signature) {
            (0, signature_1.createSignature)(rc, models_1.ReflectionKind.GetSignature, signature, symbol, getDeclaration);
        }
    }
    const setDeclaration = symbol.getDeclarations()?.find(typescript_1.default.isSetAccessor);
    if (setDeclaration) {
        const signature = context.checker.getSignatureFromDeclaration(setDeclaration);
        if (signature) {
            (0, signature_1.createSignature)(rc, models_1.ReflectionKind.SetSignature, signature, symbol, setDeclaration);
        }
    }
}
function isInherited(context, symbol) {
    const parentSymbol = context.project.getSymbolFromReflection(context.scope);
    (0, assert_1.default)(parentSymbol, `No parent symbol found for ${symbol.name} in ${context.scope.name}`);
    const parents = parentSymbol.declarations?.slice() || [];
    const constructorDecls = parents.flatMap((parent) => typescript_1.default.isClassDeclaration(parent)
        ? parent.members.filter(typescript_1.default.isConstructorDeclaration)
        : []);
    parents.push(...constructorDecls);
    return (parents.some((d) => symbol.getDeclarations()?.some((d2) => d2.parent === d)) === false);
}
function setModifiers(symbol, declaration, reflection) {
    if (!declaration) {
        return;
    }
    const modifiers = typescript_1.default.getCombinedModifierFlags(declaration);
    if (typescript_1.default.isMethodDeclaration(declaration) ||
        typescript_1.default.isPropertyDeclaration(declaration) ||
        typescript_1.default.isAccessor(declaration)) {
        if (typescript_1.default.isPrivateIdentifier(declaration.name)) {
            reflection.setFlag(models_1.ReflectionFlag.Private);
        }
    }
    if ((0, enum_1.hasAllFlags)(modifiers, typescript_1.default.ModifierFlags.Private)) {
        reflection.setFlag(models_1.ReflectionFlag.Private);
    }
    if ((0, enum_1.hasAllFlags)(modifiers, typescript_1.default.ModifierFlags.Protected)) {
        reflection.setFlag(models_1.ReflectionFlag.Protected);
    }
    if ((0, enum_1.hasAllFlags)(modifiers, typescript_1.default.ModifierFlags.Public)) {
        reflection.setFlag(models_1.ReflectionFlag.Public);
    }
    reflection.setFlag(models_1.ReflectionFlag.Optional, (0, enum_1.hasAllFlags)(symbol.flags, typescript_1.default.SymbolFlags.Optional));
    reflection.setFlag(models_1.ReflectionFlag.Readonly, (0, enum_1.hasAllFlags)(typescript_1.default.getCheckFlags(symbol), typescript_1.default.CheckFlags.Readonly) ||
        (0, enum_1.hasAllFlags)(modifiers, typescript_1.default.ModifierFlags.Readonly));
    reflection.setFlag(models_1.ReflectionFlag.Abstract, (0, enum_1.hasAllFlags)(modifiers, typescript_1.default.ModifierFlags.Abstract));
    if (reflection.kindOf(models_1.ReflectionKind.Variable) &&
        (0, enum_1.hasAllFlags)(symbol.flags, typescript_1.default.SymbolFlags.BlockScopedVariable)) {
        reflection.setFlag(models_1.ReflectionFlag.Const, (0, enum_1.hasAllFlags)(declaration.parent.flags, typescript_1.default.NodeFlags.Const));
    }
    // ReflectionFlag.Static happens when constructing the reflection.
    // We don't have sufficient information here to determine if it ought to be static.
}
