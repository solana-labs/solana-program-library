import { IOnigLib } from './onigLib';
import { IRawGrammar } from './rawGrammar';
import { IRawTheme, ScopeName } from './theme';
import { StandardTokenType } from './encodedTokenAttributes';
export * from './onigLib';
export { IRawTheme } from './theme';
/**
 * A registry helper that can locate grammar file paths given scope names.
 */
export interface RegistryOptions {
    onigLib: Promise<IOnigLib>;
    theme?: IRawTheme;
    colorMap?: string[];
    loadGrammar(scopeName: ScopeName): Promise<IRawGrammar | undefined | null>;
    getInjections?(scopeName: ScopeName): ScopeName[] | undefined;
}
/**
 * A map from scope name to a language id. Please do not use language id 0.
 */
export interface IEmbeddedLanguagesMap {
    [scopeName: string]: number;
}
/**
 * A map from selectors to token types.
 */
export interface ITokenTypeMap {
    [selector: string]: StandardTokenType;
}
export interface IGrammarConfiguration {
    embeddedLanguages?: IEmbeddedLanguagesMap;
    tokenTypes?: ITokenTypeMap;
    balancedBracketSelectors?: string[];
    unbalancedBracketSelectors?: string[];
}
/**
 * The registry that will hold all grammars.
 */
export declare class Registry {
    private readonly _options;
    private readonly _syncRegistry;
    private readonly _ensureGrammarCache;
    constructor(options: RegistryOptions);
    dispose(): void;
    /**
     * Change the theme. Once called, no previous `ruleStack` should be used anymore.
     */
    setTheme(theme: IRawTheme, colorMap?: string[]): void;
    /**
     * Returns a lookup array for color ids.
     */
    getColorMap(): string[];
    /**
     * Load the grammar for `scopeName` and all referenced included grammars asynchronously.
     * Please do not use language id 0.
     */
    loadGrammarWithEmbeddedLanguages(initialScopeName: ScopeName, initialLanguage: number, embeddedLanguages: IEmbeddedLanguagesMap): Promise<IGrammar | null>;
    /**
     * Load the grammar for `scopeName` and all referenced included grammars asynchronously.
     * Please do not use language id 0.
     */
    loadGrammarWithConfiguration(initialScopeName: ScopeName, initialLanguage: number, configuration: IGrammarConfiguration): Promise<IGrammar | null>;
    /**
     * Load the grammar for `scopeName` and all referenced included grammars asynchronously.
     */
    loadGrammar(initialScopeName: ScopeName): Promise<IGrammar | null>;
    private _loadGrammar;
    private _loadSingleGrammar;
    private _doLoadSingleGrammar;
    /**
     * Adds a rawGrammar.
     */
    addGrammar(rawGrammar: IRawGrammar, injections?: string[], initialLanguage?: number, embeddedLanguages?: IEmbeddedLanguagesMap | null): Promise<IGrammar>;
    /**
     * Get the grammar for `scopeName`. The grammar must first be created via `loadGrammar` or `addGrammar`.
     */
    private _grammarForScopeName;
}
/**
 * A grammar
 */
export interface IGrammar {
    /**
     * Tokenize `lineText` using previous line state `prevState`.
     */
    tokenizeLine(lineText: string, prevState: StateStack | null, timeLimit?: number): ITokenizeLineResult;
    /**
     * Tokenize `lineText` using previous line state `prevState`.
     * The result contains the tokens in binary format, resolved with the following information:
     *  - language
     *  - token type (regex, string, comment, other)
     *  - font style
     *  - foreground color
     *  - background color
     * e.g. for getting the languageId: `(metadata & MetadataConsts.LANGUAGEID_MASK) >>> MetadataConsts.LANGUAGEID_OFFSET`
     */
    tokenizeLine2(lineText: string, prevState: StateStack | null, timeLimit?: number): ITokenizeLineResult2;
}
export interface ITokenizeLineResult {
    readonly tokens: IToken[];
    /**
     * The `prevState` to be passed on to the next line tokenization.
     */
    readonly ruleStack: StateStack;
    /**
     * Did tokenization stop early due to reaching the time limit.
     */
    readonly stoppedEarly: boolean;
}
export interface ITokenizeLineResult2 {
    /**
     * The tokens in binary format. Each token occupies two array indices. For token i:
     *  - at offset 2*i => startIndex
     *  - at offset 2*i + 1 => metadata
     *
     */
    readonly tokens: Uint32Array;
    /**
     * The `prevState` to be passed on to the next line tokenization.
     */
    readonly ruleStack: StateStack;
    /**
     * Did tokenization stop early due to reaching the time limit.
     */
    readonly stoppedEarly: boolean;
}
export interface IToken {
    startIndex: number;
    readonly endIndex: number;
    readonly scopes: string[];
}
/**
 * **IMPORTANT** - Immutable!
 */
export interface StateStack {
    _stackElementBrand: void;
    readonly depth: number;
    clone(): StateStack;
    equals(other: StateStack): boolean;
}
export declare const INITIAL: StateStack;
export declare const parseRawGrammar: (content: string, filePath?: string) => IRawGrammar;
