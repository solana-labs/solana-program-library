import { OptionalStandardTokenType } from "../encodedTokenAttributes";
import { IEmbeddedLanguagesMap } from "../main";
import { ScopeName } from "../theme";
export declare class BasicScopeAttributes {
    readonly languageId: number;
    readonly tokenType: OptionalStandardTokenType;
    constructor(languageId: number, tokenType: OptionalStandardTokenType);
}
export declare class BasicScopeAttributesProvider {
    private readonly _defaultAttributes;
    private readonly _embeddedLanguagesMatcher;
    constructor(initialLanguageId: number, embeddedLanguages: IEmbeddedLanguagesMap | null);
    getDefaultAttributes(): BasicScopeAttributes;
    getBasicScopeAttributes(scopeName: ScopeName | null): BasicScopeAttributes;
    private static readonly _NULL_SCOPE_METADATA;
    private readonly _getBasicScopeAttributes;
    /**
     * Given a produced TM scope, return the language that token describes or null if unknown.
     * e.g. source.html => html, source.css.embedded.html => css, punctuation.definition.tag.html => null
     */
    private _scopeToLanguage;
    private _toStandardTokenType;
    private static STANDARD_TOKEN_TYPE_REGEXP;
}
