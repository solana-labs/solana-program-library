import ts from "typescript";
import { ReflectionKind } from "../../models";
import { Logger } from "../../utils";
import { CommentStyle } from "../../utils/options/declaration";
export interface DiscoveredComment {
    file: ts.SourceFile;
    ranges: ts.CommentRange[];
    jsDoc: ts.JSDoc | undefined;
}
export declare function discoverFileComment(node: ts.SourceFile, commentStyle: CommentStyle): {
    file: ts.SourceFile;
    ranges: ts.CommentRange[];
    jsDoc: ts.JSDoc | undefined;
} | undefined;
export declare function discoverComment(symbol: ts.Symbol, kind: ReflectionKind, logger: Logger, commentStyle: CommentStyle): DiscoveredComment | undefined;
export declare function discoverSignatureComment(declaration: ts.SignatureDeclaration | ts.JSDocSignature, commentStyle: CommentStyle): DiscoveredComment | undefined;
