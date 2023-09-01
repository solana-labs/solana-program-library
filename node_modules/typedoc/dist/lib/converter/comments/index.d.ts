import ts from "typescript";
import { Comment, ReflectionKind } from "../../models";
import { Logger } from "../../utils";
import type { CommentStyle, JsDocCompatibility } from "../../utils/options/declaration";
export interface CommentParserConfig {
    blockTags: Set<string>;
    inlineTags: Set<string>;
    modifierTags: Set<string>;
    jsDocCompatibility: JsDocCompatibility;
}
export declare function clearCommentCache(): void;
export declare function getComment(symbol: ts.Symbol, kind: ReflectionKind, config: CommentParserConfig, logger: Logger, commentStyle: CommentStyle, checker: ts.TypeChecker | undefined): Comment | undefined;
export declare function getFileComment(file: ts.SourceFile, config: CommentParserConfig, logger: Logger, commentStyle: CommentStyle, checker: ts.TypeChecker | undefined): Comment | undefined;
export declare function getSignatureComment(declaration: ts.SignatureDeclaration | ts.JSDocSignature, config: CommentParserConfig, logger: Logger, commentStyle: CommentStyle, checker: ts.TypeChecker | undefined): Comment | undefined;
export declare function getJsDocComment(declaration: ts.JSDocPropertyLikeTag | ts.JSDocCallbackTag | ts.JSDocTypedefTag | ts.JSDocTemplateTag | ts.JSDocEnumTag, config: CommentParserConfig, logger: Logger, checker: ts.TypeChecker | undefined): Comment | undefined;
