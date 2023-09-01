import type { Reflection } from "../reflections";
import { ReflectionSymbolId } from "../reflections/ReflectionSymbolId";
import type { Serializer, Deserializer, JSONOutput } from "../../serialization";
export type CommentDisplayPart = {
    kind: "text";
    text: string;
} | {
    kind: "code";
    text: string;
} | InlineTagDisplayPart;
/**
 * The `@link`, `@linkcode`, and `@linkplain` tags may have a `target`
 * property set indicating which reflection/url they link to. They may also
 * have a `tsLinkText` property which includes the part of the `text` which
 * TypeScript thinks should be displayed as the link text.
 */
export interface InlineTagDisplayPart {
    kind: "inline-tag";
    tag: `@${string}`;
    text: string;
    target?: Reflection | string | ReflectionSymbolId;
    tsLinkText?: string;
}
/**
 * A model that represents a single TypeDoc comment tag.
 *
 * Tags are stored in the {@link Comment.blockTags} property.
 */
export declare class CommentTag {
    /**
     * The name of this tag, e.g. `@returns`, `@example`
     */
    tag: `@${string}`;
    /**
     * Some tags, (`@typedef`, `@param`, `@property`, etc.) may have a user defined identifier associated with them.
     * If this tag is one of those, it will be parsed out and included here.
     */
    name?: string;
    /**
     * The actual body text of this tag.
     */
    content: CommentDisplayPart[];
    /**
     * Create a new CommentTag instance.
     */
    constructor(tag: `@${string}`, text: CommentDisplayPart[]);
    clone(): CommentTag;
    toObject(serializer: Serializer): JSONOutput.CommentTag;
    fromObject(de: Deserializer, obj: JSONOutput.CommentTag): void;
}
/**
 * A model that represents a comment.
 *
 * Instances of this model are created by the CommentPlugin. You can retrieve comments
 * through the {@link DeclarationReflection.comment} property.
 */
export declare class Comment {
    /**
     * Debugging utility for combining parts into a simple string. Not suitable for
     * rendering, but can be useful in tests.
     */
    static combineDisplayParts(parts: readonly CommentDisplayPart[] | undefined): string;
    /**
     * Helper function to convert an array of comment display parts into markdown suitable for
     * passing into Marked. `urlTo` will be used to resolve urls to any reflections linked to with
     * `@link` tags.
     */
    static displayPartsToMarkdown(parts: readonly CommentDisplayPart[], urlTo: (ref: Reflection) => string): string;
    /**
     * Helper utility to clone {@link Comment.summary} or {@link CommentTag.content}
     */
    static cloneDisplayParts(parts: CommentDisplayPart[]): ({
        kind: "text";
        text: string;
    } | {
        kind: "code";
        text: string;
    } | {
        kind: "inline-tag";
        tag: `@${string}`;
        text: string;
        target?: string | Reflection | ReflectionSymbolId | undefined;
        tsLinkText?: string | undefined;
    })[];
    static serializeDisplayParts(serializer: Serializer, parts: CommentDisplayPart[]): JSONOutput.CommentDisplayPart[];
    /** @hidden no point in showing this signature in api docs */
    static serializeDisplayParts(serializer: Serializer, parts: CommentDisplayPart[] | undefined): JSONOutput.CommentDisplayPart[] | undefined;
    static deserializeDisplayParts(de: Deserializer, parts: JSONOutput.CommentDisplayPart[]): CommentDisplayPart[];
    /**
     * The content of the comment which is not associated with a block tag.
     */
    summary: CommentDisplayPart[];
    /**
     * All associated block level tags.
     */
    blockTags: CommentTag[];
    /**
     * All modifier tags present on the comment, e.g. `@alpha`, `@beta`.
     */
    modifierTags: Set<string>;
    /**
     * Label associated with this reflection, if any (https://tsdoc.org/pages/tags/label/)
     */
    label?: string;
    /**
     * Creates a new Comment instance.
     */
    constructor(summary?: CommentDisplayPart[], blockTags?: CommentTag[], modifierTags?: Set<string>);
    /**
     * Create a deep clone of this comment.
     */
    clone(): Comment;
    /**
     * Returns true if this comment is completely empty.
     * @internal
     */
    isEmpty(): boolean;
    /**
     * Has this comment a visible component?
     *
     * @returns TRUE when this comment has a visible component.
     */
    hasVisibleComponent(): boolean;
    /**
     * Test whether this comment contains a tag with the given name.
     *
     * @param tagName  The name of the tag to look for.
     * @returns TRUE when this comment contains a tag with the given name, otherwise FALSE.
     */
    hasModifier(tagName: `@${string}`): boolean;
    removeModifier(tagName: `@${string}`): void;
    /**
     * Return the first tag with the given name.
     *
     * @param tagName  The name of the tag to look for.
     * @param paramName  An optional parameter name to look for.
     * @returns The found tag or undefined.
     */
    getTag(tagName: `@${string}`): CommentTag | undefined;
    /**
     * Get all tags with the given tag name.
     */
    getTags(tagName: `@${string}`): CommentTag[];
    getIdentifiedTag(identifier: string, tagName: `@${string}`): CommentTag | undefined;
    /**
     * Removes all block tags with the given tag name from the comment.
     * @param tagName
     */
    removeTags(tagName: `@${string}`): void;
    toObject(serializer: Serializer): JSONOutput.Comment;
    fromObject(de: Deserializer, obj: JSONOutput.Comment): void;
}
