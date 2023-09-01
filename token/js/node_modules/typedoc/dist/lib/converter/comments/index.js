"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.getJsDocComment = exports.getSignatureComment = exports.getFileComment = exports.getComment = exports.clearCommentCache = void 0;
const typescript_1 = __importDefault(require("typescript"));
const models_1 = require("../../models");
const utils_1 = require("../../utils");
const blockLexer_1 = require("./blockLexer");
const discovery_1 = require("./discovery");
const lineLexer_1 = require("./lineLexer");
const parser_1 = require("./parser");
const jsDocCommentKinds = [
    typescript_1.default.SyntaxKind.JSDocPropertyTag,
    typescript_1.default.SyntaxKind.JSDocCallbackTag,
    typescript_1.default.SyntaxKind.JSDocTypedefTag,
    typescript_1.default.SyntaxKind.JSDocTemplateTag,
    typescript_1.default.SyntaxKind.JSDocEnumTag,
];
let commentCache = new WeakMap();
// We need to do this for tests so that changing the tsLinkResolution option
// actually works. Without it, we'd get the old parsed comment which doesn't
// have the TS symbols attached.
function clearCommentCache() {
    commentCache = new WeakMap();
}
exports.clearCommentCache = clearCommentCache;
function getCommentWithCache(discovered, config, logger, checker) {
    if (!discovered)
        return;
    const { file, ranges, jsDoc } = discovered;
    const cache = commentCache.get(file) || new Map();
    if (cache?.has(ranges[0].pos)) {
        return cache.get(ranges[0].pos).clone();
    }
    let comment;
    switch (ranges[0].kind) {
        case typescript_1.default.SyntaxKind.MultiLineCommentTrivia:
            comment = (0, parser_1.parseComment)((0, blockLexer_1.lexBlockComment)(file.text, ranges[0].pos, ranges[0].end, jsDoc, checker), config, file, logger);
            break;
        case typescript_1.default.SyntaxKind.SingleLineCommentTrivia:
            comment = (0, parser_1.parseComment)((0, lineLexer_1.lexLineComments)(file.text, ranges), config, file, logger);
            break;
        default:
            (0, utils_1.assertNever)(ranges[0].kind);
    }
    cache.set(ranges[0].pos, comment);
    commentCache.set(file, cache);
    return comment.clone();
}
function getCommentImpl(commentSource, config, logger, moduleComment, checker) {
    const comment = getCommentWithCache(commentSource, config, logger, checker);
    if (moduleComment && comment) {
        // Module comment, make sure it is tagged with @packageDocumentation or @module.
        // If it isn't then the comment applies to the first statement in the file, so throw it away.
        if (!comment.hasModifier("@packageDocumentation") &&
            !comment.getTag("@module")) {
            return;
        }
    }
    if (!moduleComment && comment) {
        // Ensure module comments are not attached to non-module reflections.
        if (comment.hasModifier("@packageDocumentation") ||
            comment.getTag("@module")) {
            return;
        }
    }
    return comment;
}
function getComment(symbol, kind, config, logger, commentStyle, checker) {
    const declarations = symbol.declarations || [];
    if (declarations.length &&
        declarations.every((d) => jsDocCommentKinds.includes(d.kind))) {
        return getJsDocComment(declarations[0], config, logger, checker);
    }
    const comment = getCommentImpl((0, discovery_1.discoverComment)(symbol, kind, logger, commentStyle), config, logger, declarations.some(typescript_1.default.isSourceFile), checker);
    if (!comment && kind === models_1.ReflectionKind.Property) {
        return getConstructorParamPropertyComment(symbol, config, logger, commentStyle, checker);
    }
    return comment;
}
exports.getComment = getComment;
function getFileComment(file, config, logger, commentStyle, checker) {
    return getCommentImpl((0, discovery_1.discoverFileComment)(file, commentStyle), config, logger, 
    /* moduleComment */ true, checker);
}
exports.getFileComment = getFileComment;
function getConstructorParamPropertyComment(symbol, config, logger, commentStyle, checker) {
    const decl = symbol.declarations?.find(typescript_1.default.isParameter);
    if (!decl)
        return;
    const ctor = decl.parent;
    const comment = getSignatureComment(ctor, config, logger, commentStyle, checker);
    const paramTag = comment?.getIdentifiedTag(symbol.name, "@param");
    if (paramTag) {
        return new models_1.Comment(paramTag.content);
    }
}
function getSignatureComment(declaration, config, logger, commentStyle, checker) {
    return getCommentImpl((0, discovery_1.discoverSignatureComment)(declaration, commentStyle), config, logger, false, checker);
}
exports.getSignatureComment = getSignatureComment;
function getJsDocComment(declaration, config, logger, checker) {
    const file = declaration.getSourceFile();
    // First, get the whole comment. We know we'll need all of it.
    let parent = declaration.parent;
    while (!typescript_1.default.isJSDoc(parent)) {
        parent = parent.parent;
    }
    // Then parse it.
    const comment = getCommentWithCache({
        file,
        ranges: [
            {
                kind: typescript_1.default.SyntaxKind.MultiLineCommentTrivia,
                pos: parent.pos,
                end: parent.end,
            },
        ],
        jsDoc: parent,
    }, config, logger, checker);
    // And pull out the tag we actually care about.
    if (typescript_1.default.isJSDocEnumTag(declaration)) {
        return new models_1.Comment(comment.getTag("@enum")?.content);
    }
    if (typescript_1.default.isJSDocTemplateTag(declaration) &&
        declaration.comment &&
        declaration.typeParameters.length > 1) {
        // We could just put the same comment on everything, but due to how comment parsing works,
        // we'd have to search for any @template with a name starting with the first type parameter's name
        // which feels horribly hacky.
        logger.warn(`TypeDoc does not support multiple type parameters defined in a single @template tag with a comment.`, declaration);
        return;
    }
    let name;
    if (typescript_1.default.isJSDocTemplateTag(declaration)) {
        // This isn't really ideal.
        name = declaration.typeParameters[0].name.text;
    }
    else {
        name = declaration.name?.getText();
    }
    if (!name) {
        return;
    }
    const tag = comment.getIdentifiedTag(name, `@${declaration.tagName.text}`);
    if (!tag) {
        logger.error(`Failed to find JSDoc tag for ${name} after parsing comment, please file a bug report.`, declaration);
    }
    else {
        return new models_1.Comment(models_1.Comment.cloneDisplayParts(tag.content));
    }
}
exports.getJsDocComment = getJsDocComment;
