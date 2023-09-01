import type { CommentParserConfig } from ".";
import { Comment } from "../../models";
import { Logger } from "../../utils";
import type { MinimalSourceFile } from "../../utils/minimalSourceFile";
import { Token } from "./lexer";
export declare function parseComment(tokens: Generator<Token, undefined, undefined>, config: CommentParserConfig, file: MinimalSourceFile, logger: Logger): Comment;
