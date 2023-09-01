"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.convertIndexSignature = void 0;
const assert_1 = __importDefault(require("assert"));
const typescript_1 = __importDefault(require("typescript"));
const models_1 = require("../../models");
const converter_events_1 = require("../converter-events");
function convertIndexSignature(context, symbol) {
    (0, assert_1.default)(context.scope instanceof models_1.DeclarationReflection);
    const indexSymbol = symbol.members?.get("__index");
    if (indexSymbol) {
        // Right now TypeDoc models don't have a way to distinguish between string
        // and number index signatures... { [x: string]: 1 | 2; [x: number]: 2 }
        // will be misrepresented.
        const indexDeclaration = indexSymbol.getDeclarations()?.[0];
        (0, assert_1.default)(indexDeclaration && typescript_1.default.isIndexSignatureDeclaration(indexDeclaration));
        const param = indexDeclaration.parameters[0];
        (0, assert_1.default)(param && typescript_1.default.isParameter(param));
        const index = new models_1.SignatureReflection("__index", models_1.ReflectionKind.IndexSignature, context.scope);
        index.parameters = [
            new models_1.ParameterReflection(param.name.getText(), models_1.ReflectionKind.Parameter, index),
        ];
        index.parameters[0].type = context.converter.convertType(context.withScope(index.parameters[0]), param.type);
        index.type = context.converter.convertType(context.withScope(index), indexDeclaration.type);
        context.registerReflection(index, indexSymbol);
        context.scope.indexSignature = index;
        context.trigger(converter_events_1.ConverterEvents.CREATE_SIGNATURE, index, indexDeclaration);
    }
}
exports.convertIndexSignature = convertIndexSignature;
