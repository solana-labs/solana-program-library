import { OrMask } from './utils';
export declare class Theme {
    private readonly _colorMap;
    private readonly _defaults;
    private readonly _root;
    static createFromRawTheme(source: IRawTheme | undefined, colorMap?: string[]): Theme;
    static createFromParsedTheme(source: ParsedThemeRule[], colorMap?: string[]): Theme;
    private readonly _cachedMatchRoot;
    constructor(_colorMap: ColorMap, _defaults: StyleAttributes, _root: ThemeTrieElement);
    getColorMap(): string[];
    getDefaults(): StyleAttributes;
    match(scopePath: ScopeStack | null): StyleAttributes | null;
}
/**
 * Identifiers with a binary dot operator.
 * Examples: `baz` or `foo.bar`
*/
export declare type ScopeName = string;
/**
 * An expression language of ScopeNames with a binary space (to indicate nesting) operator.
 * Examples: `foo.bar boo.baz`
*/
export declare type ScopePath = string;
/**
 * An expression language of ScopePathStr with a binary comma (to indicate alternatives) operator.
 * Examples: `foo.bar boo.baz,quick quack`
*/
export declare type ScopePattern = string;
/**
 * A TextMate theme.
 */
export interface IRawTheme {
    readonly name?: string;
    readonly settings: IRawThemeSetting[];
}
/**
 * A single theme setting.
 */
export interface IRawThemeSetting {
    readonly name?: string;
    readonly scope?: ScopePattern | ScopePattern[];
    readonly settings: {
        readonly fontStyle?: string;
        readonly foreground?: string;
        readonly background?: string;
    };
}
export declare class ScopeStack {
    readonly parent: ScopeStack | null;
    readonly scopeName: ScopeName;
    static from(first: ScopeName, ...segments: ScopeName[]): ScopeStack;
    static from(...segments: ScopeName[]): ScopeStack | null;
    constructor(parent: ScopeStack | null, scopeName: ScopeName);
    push(scopeName: ScopeName): ScopeStack;
    getSegments(): ScopeName[];
    toString(): string;
}
export declare class StyleAttributes {
    readonly fontStyle: OrMask<FontStyle>;
    readonly foregroundId: number;
    readonly backgroundId: number;
    constructor(fontStyle: OrMask<FontStyle>, foregroundId: number, backgroundId: number);
}
/**
 * Parse a raw theme into rules.
 */
export declare function parseTheme(source: IRawTheme | undefined): ParsedThemeRule[];
export declare class ParsedThemeRule {
    readonly scope: ScopeName;
    readonly parentScopes: ScopeName[] | null;
    readonly index: number;
    readonly fontStyle: OrMask<FontStyle>;
    readonly foreground: string | null;
    readonly background: string | null;
    constructor(scope: ScopeName, parentScopes: ScopeName[] | null, index: number, fontStyle: OrMask<FontStyle>, foreground: string | null, background: string | null);
}
export declare const enum FontStyle {
    NotSet = -1,
    None = 0,
    Italic = 1,
    Bold = 2,
    Underline = 4,
    Strikethrough = 8
}
export declare function fontStyleToString(fontStyle: OrMask<FontStyle>): string;
export declare class ColorMap {
    private readonly _isFrozen;
    private _lastColorId;
    private _id2color;
    private _color2id;
    constructor(_colorMap?: string[]);
    getId(color: string | null): number;
    getColorMap(): string[];
}
export declare class ThemeTrieElementRule {
    scopeDepth: number;
    parentScopes: ScopeName[] | null;
    fontStyle: number;
    foreground: number;
    background: number;
    constructor(scopeDepth: number, parentScopes: ScopeName[] | null, fontStyle: number, foreground: number, background: number);
    clone(): ThemeTrieElementRule;
    static cloneArr(arr: ThemeTrieElementRule[]): ThemeTrieElementRule[];
    acceptOverwrite(scopeDepth: number, fontStyle: number, foreground: number, background: number): void;
}
export interface ITrieChildrenMap {
    [segment: string]: ThemeTrieElement;
}
export declare class ThemeTrieElement {
    private readonly _mainRule;
    private readonly _children;
    private readonly _rulesWithParentScopes;
    constructor(_mainRule: ThemeTrieElementRule, rulesWithParentScopes?: ThemeTrieElementRule[], _children?: ITrieChildrenMap);
    private static _sortBySpecificity;
    private static _cmpBySpecificity;
    match(scope: ScopeName): ThemeTrieElementRule[];
    insert(scopeDepth: number, scope: ScopeName, parentScopes: ScopeName[] | null, fontStyle: number, foreground: number, background: number): void;
    private _doInsertHere;
}
