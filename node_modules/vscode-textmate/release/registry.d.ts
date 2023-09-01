import { BalancedBracketSelectors, IGrammarRepository, IThemeProvider } from './grammar';
import { IRawGrammar } from './rawGrammar';
import { IGrammar, IEmbeddedLanguagesMap, ITokenTypeMap } from './main';
import { ScopeStack, Theme, StyleAttributes, ScopeName } from './theme';
import { IOnigLib } from './onigLib';
export declare class SyncRegistry implements IGrammarRepository, IThemeProvider {
    private readonly _onigLibPromise;
    private readonly _grammars;
    private readonly _rawGrammars;
    private readonly _injectionGrammars;
    private _theme;
    constructor(theme: Theme, _onigLibPromise: Promise<IOnigLib>);
    dispose(): void;
    setTheme(theme: Theme): void;
    getColorMap(): string[];
    /**
     * Add `grammar` to registry and return a list of referenced scope names
     */
    addGrammar(grammar: IRawGrammar, injectionScopeNames?: ScopeName[]): void;
    /**
     * Lookup a raw grammar.
     */
    lookup(scopeName: ScopeName): IRawGrammar | undefined;
    /**
     * Returns the injections for the given grammar
     */
    injections(targetScope: ScopeName): ScopeName[];
    /**
     * Get the default theme settings
     */
    getDefaults(): StyleAttributes;
    /**
     * Match a scope in the theme.
     */
    themeMatch(scopePath: ScopeStack): StyleAttributes | null;
    /**
     * Lookup a grammar.
     */
    grammarForScopeName(scopeName: ScopeName, initialLanguage: number, embeddedLanguages: IEmbeddedLanguagesMap | null, tokenTypes: ITokenTypeMap | null, balancedBracketSelectors: BalancedBracketSelectors | null): Promise<IGrammar | null>;
}
