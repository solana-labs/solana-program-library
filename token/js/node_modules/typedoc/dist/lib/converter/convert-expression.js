"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.convertExpression = exports.convertDefaultValue = void 0;
const typescript_1 = __importDefault(require("typescript"));
/**
 * Return the default value of the given node.
 *
 * @param node  The TypeScript node whose default value should be extracted.
 * @returns The default value as a string.
 */
function convertDefaultValue(node) {
    const anyNode = node;
    if (anyNode?.initializer) {
        return convertExpression(anyNode.initializer);
    }
    else {
        return undefined;
    }
}
exports.convertDefaultValue = convertDefaultValue;
function convertExpression(expression) {
    switch (expression.kind) {
        case typescript_1.default.SyntaxKind.StringLiteral:
        case typescript_1.default.SyntaxKind.TrueKeyword:
        case typescript_1.default.SyntaxKind.FalseKeyword:
        case typescript_1.default.SyntaxKind.NullKeyword:
        case typescript_1.default.SyntaxKind.NumericLiteral:
        case typescript_1.default.SyntaxKind.PrefixUnaryExpression:
        case typescript_1.default.SyntaxKind.Identifier:
            return expression.getText();
    }
    if (typescript_1.default.isArrayLiteralExpression(expression) &&
        expression.elements.length === 0) {
        return "[]";
    }
    if (typescript_1.default.isObjectLiteralExpression(expression) &&
        expression.properties.length === 0) {
        return "{}";
    }
    // a.b.c.d
    if (typescript_1.default.isPropertyAccessExpression(expression)) {
        const parts = [expression.name.getText()];
        let iter = expression.expression;
        while (typescript_1.default.isPropertyAccessExpression(iter)) {
            parts.unshift(iter.name.getText());
            iter = iter.expression;
        }
        if (typescript_1.default.isIdentifier(iter)) {
            parts.unshift(iter.text);
            return parts.join(".");
        }
    }
    // More complex expressions are generally not useful in the documentation.
    // Show that there was a value, but not specifics.
    return "...";
}
exports.convertExpression = convertExpression;
