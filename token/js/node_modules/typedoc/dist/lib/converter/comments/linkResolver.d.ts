import { Comment, CommentDisplayPart, Reflection, ReflectionSymbolId } from "../../models";
import { DeclarationReference } from "./declarationReference";
export type ExternalResolveResult = {
    target: string;
    caption?: string;
};
/**
 * @param ref - Parsed declaration reference to resolve. This may be created automatically for some symbol, or
 *   parsed from user input.
 * @param refl - Reflection that contains the resolved link
 * @param part - If the declaration reference was created from a comment, the originating part.
 * @param symbolId - If the declaration reference was created from a symbol, or `useTsLinkResolution` is turned
 *   on and TypeScript resolved the link to some symbol, the ID of that symbol.
 */
export type ExternalSymbolResolver = (ref: DeclarationReference, refl: Reflection, part: Readonly<CommentDisplayPart> | undefined, symbolId: ReflectionSymbolId | undefined) => ExternalResolveResult | string | undefined;
export declare function resolveLinks(comment: Comment, reflection: Reflection, externalResolver: ExternalSymbolResolver): void;
export declare function resolvePartLinks(reflection: Reflection, parts: readonly CommentDisplayPart[], externalResolver: ExternalSymbolResolver): CommentDisplayPart[];
