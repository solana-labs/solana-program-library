"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.createTypeParamReflection = exports.convertTypeParameterNodes = exports.convertParameterNodes = exports.createSignature = void 0;
const typescript_1 = __importDefault(require("typescript"));
const assert_1 = __importDefault(require("assert"));
const models_1 = require("../../models");
const converter_events_1 = require("../converter-events");
const convert_expression_1 = require("../convert-expression");
const reflections_1 = require("../utils/reflections");
const ReflectionSymbolId_1 = require("../../models/reflections/ReflectionSymbolId");
function createSignature(context, kind, signature, symbol, declaration) {
    var _a;
    (0, assert_1.default)(context.scope instanceof models_1.DeclarationReflection);
    declaration || (declaration = signature.getDeclaration());
    const sigRef = new models_1.SignatureReflection(kind == models_1.ReflectionKind.ConstructorSignature
        ? `new ${context.scope.parent.name}`
        : context.scope.name, kind, context.scope);
    const sigRefCtx = context.withScope(sigRef);
    if (symbol && declaration) {
        context.project.registerSymbolId(sigRef, new ReflectionSymbolId_1.ReflectionSymbolId(symbol, declaration));
    }
    // If we are creating signatures for a variable or property and it has a comment associated with it
    // then we should prefer that comment over any comment on the signature. The comment plugin
    // will copy the comment down if this signature doesn't have one, so don't set one.
    let parentReflection = context.scope;
    if (parentReflection.kindOf(models_1.ReflectionKind.TypeLiteral) &&
        parentReflection.parent instanceof models_1.DeclarationReflection) {
        parentReflection = parentReflection.parent;
    }
    if (declaration &&
        (!parentReflection.comment ||
            !(parentReflection.conversionFlags &
                models_1.ConversionFlags.VariableOrPropertySource))) {
        sigRef.comment = context.getSignatureComment(declaration);
    }
    sigRef.typeParameters = convertTypeParameters(sigRefCtx, sigRef, signature.typeParameters);
    const parameterSymbols = signature.thisParameter
        ? [signature.thisParameter, ...signature.parameters]
        : signature.parameters;
    sigRef.parameters = convertParameters(sigRefCtx, sigRef, parameterSymbols, declaration?.parameters);
    const predicate = context.checker.getTypePredicateOfSignature(signature);
    if (predicate) {
        sigRef.type = convertPredicate(predicate, sigRefCtx);
    }
    else if (kind == models_1.ReflectionKind.SetSignature) {
        sigRef.type = new models_1.IntrinsicType("void");
    }
    else {
        sigRef.type = context.converter.convertType(sigRefCtx, (declaration?.kind === typescript_1.default.SyntaxKind.FunctionDeclaration &&
            declaration.type) ||
            signature.getReturnType());
    }
    context.registerReflection(sigRef, undefined);
    switch (kind) {
        case models_1.ReflectionKind.GetSignature:
            context.scope.getSignature = sigRef;
            break;
        case models_1.ReflectionKind.SetSignature:
            context.scope.setSignature = sigRef;
            break;
        case models_1.ReflectionKind.CallSignature:
        case models_1.ReflectionKind.ConstructorSignature:
            (_a = context.scope).signatures ?? (_a.signatures = []);
            context.scope.signatures.push(sigRef);
            break;
    }
    context.converter.trigger(converter_events_1.ConverterEvents.CREATE_SIGNATURE, context, sigRef, declaration, signature);
}
exports.createSignature = createSignature;
function convertParameters(context, sigRef, parameters, parameterNodes) {
    return parameters.map((param, i) => {
        const declaration = param.valueDeclaration;
        (0, assert_1.default)(!declaration ||
            typescript_1.default.isParameter(declaration) ||
            typescript_1.default.isJSDocParameterTag(declaration));
        const paramRefl = new models_1.ParameterReflection(/__\d+/.test(param.name) ? "__namedParameters" : param.name, models_1.ReflectionKind.Parameter, sigRef);
        if (declaration && typescript_1.default.isJSDocParameterTag(declaration)) {
            paramRefl.comment = context.getJsDocComment(declaration);
        }
        paramRefl.comment || (paramRefl.comment = context.getComment(param, paramRefl.kind));
        context.registerReflection(paramRefl, param);
        context.trigger(converter_events_1.ConverterEvents.CREATE_PARAMETER, paramRefl);
        let type;
        if (declaration) {
            type = context.checker.getTypeOfSymbolAtLocation(param, declaration);
        }
        else {
            type = param.type;
        }
        paramRefl.type = context.converter.convertType(context.withScope(paramRefl), type);
        let isOptional = false;
        if (declaration) {
            isOptional = typescript_1.default.isParameter(declaration)
                ? !!declaration.questionToken ||
                    typescript_1.default
                        .getJSDocParameterTags(declaration)
                        .some((tag) => tag.isBracketed)
                : declaration.isBracketed;
        }
        if (isOptional) {
            paramRefl.type = (0, reflections_1.removeUndefined)(paramRefl.type);
        }
        paramRefl.defaultValue = (0, convert_expression_1.convertDefaultValue)(parameterNodes?.[i]);
        paramRefl.setFlag(models_1.ReflectionFlag.Optional, isOptional);
        // If we have no declaration, then this is an implicitly defined parameter in JS land
        // because the method body uses `arguments`... which is always a rest argument
        let isRest = true;
        if (declaration) {
            isRest = typescript_1.default.isParameter(declaration)
                ? !!declaration.dotDotDotToken
                : !!declaration.typeExpression &&
                    typescript_1.default.isJSDocVariadicType(declaration.typeExpression.type);
        }
        paramRefl.setFlag(models_1.ReflectionFlag.Rest, isRest);
        return paramRefl;
    });
}
function convertParameterNodes(context, sigRef, parameters) {
    return parameters.map((param) => {
        const paramRefl = new models_1.ParameterReflection(/__\d+/.test(param.name.getText())
            ? "__namedParameters"
            : param.name.getText(), models_1.ReflectionKind.Parameter, sigRef);
        if (typescript_1.default.isJSDocParameterTag(param)) {
            paramRefl.comment = context.getJsDocComment(param);
        }
        context.registerReflection(paramRefl, context.getSymbolAtLocation(param));
        context.trigger(converter_events_1.ConverterEvents.CREATE_PARAMETER, paramRefl);
        paramRefl.type = context.converter.convertType(context.withScope(paramRefl), typescript_1.default.isParameter(param) ? param.type : param.typeExpression?.type);
        const isOptional = typescript_1.default.isParameter(param)
            ? !!param.questionToken
            : param.isBracketed;
        if (isOptional) {
            paramRefl.type = (0, reflections_1.removeUndefined)(paramRefl.type);
        }
        paramRefl.defaultValue = (0, convert_expression_1.convertDefaultValue)(param);
        paramRefl.setFlag(models_1.ReflectionFlag.Optional, isOptional);
        paramRefl.setFlag(models_1.ReflectionFlag.Rest, typescript_1.default.isParameter(param)
            ? !!param.dotDotDotToken
            : !!param.typeExpression &&
                typescript_1.default.isJSDocVariadicType(param.typeExpression.type));
        return paramRefl;
    });
}
exports.convertParameterNodes = convertParameterNodes;
function convertTypeParameters(context, parent, parameters) {
    return parameters?.map((param) => {
        const constraintT = param.getConstraint();
        const defaultT = param.getDefault();
        // There's no way to determine directly from a ts.TypeParameter what it's variance modifiers are
        // so unfortunately we have to go back to the node for this...
        const declaration = param
            .getSymbol()
            ?.declarations?.find(typescript_1.default.isTypeParameterDeclaration);
        const variance = getVariance(declaration?.modifiers);
        const paramRefl = new models_1.TypeParameterReflection(param.symbol.name, parent, variance);
        const paramCtx = context.withScope(paramRefl);
        paramRefl.type = constraintT
            ? context.converter.convertType(paramCtx, constraintT)
            : void 0;
        paramRefl.default = defaultT
            ? context.converter.convertType(paramCtx, defaultT)
            : void 0;
        // No way to determine this from the type parameter itself, need to go back to the declaration
        if (declaration?.modifiers?.some((m) => m.kind === typescript_1.default.SyntaxKind.ConstKeyword)) {
            paramRefl.flags.setFlag(models_1.ReflectionFlag.Const, true);
        }
        context.registerReflection(paramRefl, param.getSymbol());
        context.trigger(converter_events_1.ConverterEvents.CREATE_TYPE_PARAMETER, paramRefl);
        return paramRefl;
    });
}
function convertTypeParameterNodes(context, parameters) {
    return parameters?.map((param) => createTypeParamReflection(param, context));
}
exports.convertTypeParameterNodes = convertTypeParameterNodes;
function createTypeParamReflection(param, context) {
    const paramRefl = new models_1.TypeParameterReflection(param.name.text, context.scope, getVariance(param.modifiers));
    const paramScope = context.withScope(paramRefl);
    paramRefl.type = param.constraint
        ? context.converter.convertType(paramScope, param.constraint)
        : void 0;
    paramRefl.default = param.default
        ? context.converter.convertType(paramScope, param.default)
        : void 0;
    if (param.modifiers?.some((m) => m.kind === typescript_1.default.SyntaxKind.ConstKeyword)) {
        paramRefl.flags.setFlag(models_1.ReflectionFlag.Const, true);
    }
    context.registerReflection(paramRefl, param.symbol);
    if (typescript_1.default.isJSDocTemplateTag(param.parent)) {
        paramRefl.comment = context.getJsDocComment(param.parent);
    }
    context.trigger(converter_events_1.ConverterEvents.CREATE_TYPE_PARAMETER, paramRefl, param);
    return paramRefl;
}
exports.createTypeParamReflection = createTypeParamReflection;
function getVariance(modifiers) {
    const hasIn = modifiers?.some((mod) => mod.kind === typescript_1.default.SyntaxKind.InKeyword);
    const hasOut = modifiers?.some((mod) => mod.kind === typescript_1.default.SyntaxKind.OutKeyword);
    if (hasIn && hasOut) {
        return models_1.VarianceModifier.inOut;
    }
    if (hasIn) {
        return models_1.VarianceModifier.in;
    }
    if (hasOut) {
        return models_1.VarianceModifier.out;
    }
}
function convertPredicate(predicate, context) {
    let name;
    switch (predicate.kind) {
        case typescript_1.default.TypePredicateKind.This:
        case typescript_1.default.TypePredicateKind.AssertsThis:
            name = "this";
            break;
        case typescript_1.default.TypePredicateKind.Identifier:
        case typescript_1.default.TypePredicateKind.AssertsIdentifier:
            name = predicate.parameterName;
            break;
    }
    let asserts;
    switch (predicate.kind) {
        case typescript_1.default.TypePredicateKind.This:
        case typescript_1.default.TypePredicateKind.Identifier:
            asserts = false;
            break;
        case typescript_1.default.TypePredicateKind.AssertsThis:
        case typescript_1.default.TypePredicateKind.AssertsIdentifier:
            asserts = true;
            break;
    }
    return new models_1.PredicateType(name, asserts, predicate.type
        ? context.converter.convertType(context, predicate.type)
        : void 0);
}
