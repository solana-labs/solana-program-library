import type * as ts from "typescript";
import { Token } from "./lexer";
export declare function lexLineComments(file: string, ranges: ts.CommentRange[]): Generator<Token, undefined, undefined>;
