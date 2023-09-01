"use strict";
// Converter functions for JSDoc defined types
// @typedef
// @callback
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.convertJsDocCallback = exports.convertJsDocAlias = void 0;
const assert_1 = require("assert");
const typescript_1 = __importDefault(require("typescript"));
const models_1 = require("../models");
const ReflectionSymbolId_1 = require("../models/reflections/ReflectionSymbolId");
const converter_events_1 = require("./converter-events");
const signature_1 = require("./factories/signature");
function convertJsDocAlias(context, symbol, declaration, exportSymbol) {
    if (declaration.typeExpression &&
        typescript_1.default.isJSDocTypeLiteral(declaration.typeExpression)) {
        convertJsDocInterface(context, declaration, symbol, exportSymbol);
        return;
    }
    // If the typedef tag is just referring to another type-space symbol, with no type parameters
    // or appropriate forwarding type parameters, then we treat it as a re-export instead of creating
    // a type alias with an import type.
    const aliasedSymbol = getTypedefReExportTarget(context, declaration);
    if (aliasedSymbol) {
        context.converter.convertSymbol(context, aliasedSymbol, exportSymbol ?? symbol);
        return;
    }
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.TypeAlias, symbol, exportSymbol);
    reflection.comment = context.getJsDocComment(declaration);
    reflection.type = context.converter.convertType(context.withScope(reflection), declaration.typeExpression?.type);
    convertTemplateParameters(context.withScope(reflection), declaration.parent);
    context.finalizeDeclarationReflection(reflection);
}
exports.convertJsDocAlias = convertJsDocAlias;
function convertJsDocCallback(context, symbol, declaration, exportSymbol) {
    const alias = context.createDeclarationReflection(models_1.ReflectionKind.TypeAlias, symbol, exportSymbol);
    alias.comment = context.getJsDocComment(declaration);
    context.finalizeDeclarationReflection(alias);
    const ac = context.withScope(alias);
    alias.type = convertJsDocSignature(ac, declaration.typeExpression);
    convertTemplateParameters(ac, declaration.parent);
}
exports.convertJsDocCallback = convertJsDocCallback;
function convertJsDocInterface(context, declaration, symbol, exportSymbol) {
    const reflection = context.createDeclarationReflection(models_1.ReflectionKind.Interface, symbol, exportSymbol);
    reflection.comment = context.getJsDocComment(declaration);
    context.finalizeDeclarationReflection(reflection);
    const rc = context.withScope(reflection);
    const type = context.checker.getDeclaredTypeOfSymbol(symbol);
    for (const s of type.getProperties()) {
        context.converter.convertSymbol(rc, s);
    }
    convertTemplateParameters(rc, declaration.parent);
}
function convertJsDocSignature(context, node) {
    const symbol = context.getSymbolAtLocation(node) ?? node.symbol;
    const type = context.getTypeAtLocation(node);
    if (!symbol || !type) {
        return new models_1.IntrinsicType("Function");
    }
    const reflection = new models_1.DeclarationReflection("__type", models_1.ReflectionKind.TypeLiteral, context.scope);
    context.registerReflection(reflection, symbol);
    context.trigger(converter_events_1.ConverterEvents.CREATE_DECLARATION, reflection);
    const signature = new models_1.SignatureReflection("__type", models_1.ReflectionKind.CallSignature, reflection);
    context.project.registerSymbolId(signature, new ReflectionSymbolId_1.ReflectionSymbolId(symbol, node));
    context.registerReflection(signature, void 0);
    const signatureCtx = context.withScope(signature);
    reflection.signatures = [signature];
    signature.type = context.converter.convertType(signatureCtx, node.type?.typeExpression?.type);
    signature.parameters = (0, signature_1.convertParameterNodes)(signatureCtx, signature, node.parameters);
    signature.typeParameters = convertTemplateParameterNodes(context.withScope(reflection), node.typeParameters);
    return new models_1.ReflectionType(reflection);
}
function convertTemplateParameters(context, node) {
    (0, assert_1.ok)(context.scope instanceof models_1.DeclarationReflection);
    context.scope.typeParameters = convertTemplateParameterNodes(context, node.tags?.filter(typescript_1.default.isJSDocTemplateTag));
}
function convertTemplateParameterNodes(context, nodes) {
    const params = (nodes ?? []).flatMap((tag) => tag.typeParameters);
    return (0, signature_1.convertTypeParameterNodes)(context, params);
}
function getTypedefReExportTarget(context, declaration) {
    const typeExpression = declaration.typeExpression;
    if (!typescript_1.default.isJSDocTypedefTag(declaration) ||
        !typeExpression ||
        typescript_1.default.isJSDocTypeLiteral(typeExpression) ||
        !typescript_1.default.isImportTypeNode(typeExpression.type) ||
        !typeExpression.type.qualifier ||
        !typescript_1.default.isIdentifier(typeExpression.type.qualifier)) {
        return;
    }
    const targetSymbol = context.expectSymbolAtLocation(typeExpression.type.qualifier);
    const decl = targetSymbol.declarations?.[0];
    if (!decl ||
        !(typescript_1.default.isTypeAliasDeclaration(decl) ||
            typescript_1.default.isInterfaceDeclaration(decl) ||
            typescript_1.default.isJSDocTypedefTag(decl) ||
            typescript_1.default.isJSDocCallbackTag(decl))) {
        return;
    }
    const targetParams = typescript_1.default.getEffectiveTypeParameterDeclarations(decl);
    const localParams = typescript_1.default.getEffectiveTypeParameterDeclarations(declaration);
    const localArgs = typeExpression.type.typeArguments || [];
    // If we have type parameters, ensure they are forwarding parameters with no transformations.
    // This doesn't check constraints since they aren't checked in JSDoc types.
    if (targetParams.length !== localParams.length ||
        localArgs.some((arg, i) => !typescript_1.default.isTypeReferenceNode(arg) ||
            !typescript_1.default.isIdentifier(arg.typeName) ||
            arg.typeArguments ||
            localParams[i]?.name.text !== arg.typeName.text)) {
        return;
    }
    return targetSymbol;
}
