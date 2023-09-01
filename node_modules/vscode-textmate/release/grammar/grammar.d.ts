import { EncodedTokenAttributes, StandardTokenType } from '../encodedTokenAttributes';
import { IEmbeddedLanguagesMap, IGrammar, IToken, ITokenizeLineResult, ITokenizeLineResult2, ITokenTypeMap, StateStack as StackElementDef } from '../main';
import { Matcher } from '../matcher';
import { IOnigLib, OnigScanner, OnigString } from '../onigLib';
import { IRawGrammar, IRawRepository } from '../rawGrammar';
import { IRuleFactoryHelper, IRuleRegistry, Rule, RuleId } from '../rule';
import { ScopeName, ScopePath, ScopeStack, StyleAttributes } from '../theme';
import { BasicScopeAttributes } from './basicScopesAttributeProvider';
export declare function createGrammar(scopeName: ScopeName, grammar: IRawGrammar, initialLanguage: number, embeddedLanguages: IEmbeddedLanguagesMap | null, tokenTypes: ITokenTypeMap | null, balancedBracketSelectors: BalancedBracketSelectors | null, grammarRepository: IGrammarRepository & IThemeProvider, onigLib: IOnigLib): Grammar;
export interface IThemeProvider {
    themeMatch(scopePath: ScopeStack): StyleAttributes | null;
    getDefaults(): StyleAttributes;
}
export interface IGrammarRepository {
    lookup(scopeName: ScopeName): IRawGrammar | undefined;
    injections(scopeName: ScopeName): ScopeName[];
}
export interface Injection {
    readonly debugSelector: string;
    readonly matcher: Matcher<string[]>;
    readonly priority: -1 | 0 | 1;
    readonly ruleId: RuleId;
    readonly grammar: IRawGrammar;
}
export declare class Grammar implements IGrammar, IRuleFactoryHelper, IOnigLib {
    private readonly _rootScopeName;
    private readonly balancedBracketSelectors;
    private readonly _onigLib;
    private _rootId;
    private _lastRuleId;
    private readonly _ruleId2desc;
    private readonly _includedGrammars;
    private readonly _grammarRepository;
    private readonly _grammar;
    private _injections;
    private readonly _basicScopeAttributesProvider;
    private readonly _tokenTypeMatchers;
    get themeProvider(): IThemeProvider;
    constructor(_rootScopeName: ScopeName, grammar: IRawGrammar, initialLanguage: number, embeddedLanguages: IEmbeddedLanguagesMap | null, tokenTypes: ITokenTypeMap | null, balancedBracketSelectors: BalancedBracketSelectors | null, grammarRepository: IGrammarRepository & IThemeProvider, _onigLib: IOnigLib);
    dispose(): void;
    createOnigScanner(sources: string[]): OnigScanner;
    createOnigString(sources: string): OnigString;
    getMetadataForScope(scope: string): BasicScopeAttributes;
    private _collectInjections;
    getInjections(): Injection[];
    registerRule<T extends Rule>(factory: (id: RuleId) => T): T;
    getRule(ruleId: RuleId): Rule;
    getExternalGrammar(scopeName: string, repository?: IRawRepository): IRawGrammar | undefined;
    tokenizeLine(lineText: string, prevState: StateStack | null, timeLimit?: number): ITokenizeLineResult;
    tokenizeLine2(lineText: string, prevState: StateStack | null, timeLimit?: number): ITokenizeLineResult2;
    private _tokenize;
}
export declare class AttributedScopeStack {
    readonly parent: AttributedScopeStack | null;
    readonly scopePath: ScopeStack;
    readonly tokenAttributes: EncodedTokenAttributes;
    static createRoot(scopeName: ScopeName, tokenAttributes: EncodedTokenAttributes): AttributedScopeStack;
    static createRootAndLookUpScopeName(scopeName: ScopeName, tokenAttributes: EncodedTokenAttributes, grammar: Grammar): AttributedScopeStack;
    get scopeName(): ScopeName;
    private constructor();
    equals(other: AttributedScopeStack): boolean;
    private static _equals;
    private static mergeAttributes;
    pushAttributed(scopePath: ScopePath | null, grammar: Grammar): AttributedScopeStack;
    private static _pushAttributed;
    getScopeNames(): string[];
}
/**
 * Represents a "pushed" state on the stack (as a linked list element).
 */
export declare class StateStack implements StackElementDef {
    /**
     * The previous state on the stack (or null for the root state).
     */
    readonly parent: StateStack | null;
    /**
     * The state (rule) that this element represents.
     */
    private readonly ruleId;
    /**
     * The state has entered and captured \n. This means that the next line should have an anchorPosition of 0.
     */
    readonly beginRuleCapturedEOL: boolean;
    /**
     * The "pop" (end) condition for this state in case that it was dynamically generated through captured text.
     */
    readonly endRule: string | null;
    /**
     * The list of scopes containing the "name" for this state.
     */
    readonly nameScopesList: AttributedScopeStack;
    /**
     * The list of scopes containing the "contentName" (besides "name") for this state.
     * This list **must** contain as an element `scopeName`.
     */
    readonly contentNameScopesList: AttributedScopeStack;
    _stackElementBrand: void;
    static NULL: StateStack;
    /**
     * The position on the current line where this state was pushed.
     * This is relevant only while tokenizing a line, to detect endless loops.
     * Its value is meaningless across lines.
     */
    private _enterPos;
    /**
     * The captured anchor position when this stack element was pushed.
     * This is relevant only while tokenizing a line, to restore the anchor position when popping.
     * Its value is meaningless across lines.
     */
    private _anchorPos;
    /**
     * The depth of the stack.
     */
    readonly depth: number;
    constructor(
    /**
     * The previous state on the stack (or null for the root state).
     */
    parent: StateStack | null, 
    /**
     * The state (rule) that this element represents.
     */
    ruleId: RuleId, enterPos: number, anchorPos: number, 
    /**
     * The state has entered and captured \n. This means that the next line should have an anchorPosition of 0.
     */
    beginRuleCapturedEOL: boolean, 
    /**
     * The "pop" (end) condition for this state in case that it was dynamically generated through captured text.
     */
    endRule: string | null, 
    /**
     * The list of scopes containing the "name" for this state.
     */
    nameScopesList: AttributedScopeStack, 
    /**
     * The list of scopes containing the "contentName" (besides "name") for this state.
     * This list **must** contain as an element `scopeName`.
     */
    contentNameScopesList: AttributedScopeStack);
    equals(other: StateStack): boolean;
    private static _equals;
    /**
     * A structural equals check. Does not take into account `scopes`.
     */
    private static _structuralEquals;
    clone(): StateStack;
    private static _reset;
    reset(): void;
    pop(): StateStack | null;
    safePop(): StateStack;
    push(ruleId: RuleId, enterPos: number, anchorPos: number, beginRuleCapturedEOL: boolean, endRule: string | null, nameScopesList: AttributedScopeStack, contentNameScopesList: AttributedScopeStack): StateStack;
    getEnterPos(): number;
    getAnchorPos(): number;
    getRule(grammar: IRuleRegistry): Rule;
    toString(): string;
    private _writeString;
    withContentNameScopesList(contentNameScopeStack: AttributedScopeStack): StateStack;
    withEndRule(endRule: string): StateStack;
    hasSameRuleAs(other: StateStack): boolean;
}
interface TokenTypeMatcher {
    readonly matcher: Matcher<string[]>;
    readonly type: StandardTokenType;
}
export declare class BalancedBracketSelectors {
    private readonly balancedBracketScopes;
    private readonly unbalancedBracketScopes;
    private allowAny;
    constructor(balancedBracketScopes: string[], unbalancedBracketScopes: string[]);
    get matchesAlways(): boolean;
    get matchesNever(): boolean;
    match(scopes: string[]): boolean;
}
export declare class LineTokens {
    private readonly balancedBracketSelectors;
    private readonly _emitBinaryTokens;
    /**
     * defined only if `DebugFlags.InDebugMode`.
     */
    private readonly _lineText;
    /**
     * used only if `_emitBinaryTokens` is false.
     */
    private readonly _tokens;
    /**
     * used only if `_emitBinaryTokens` is true.
     */
    private readonly _binaryTokens;
    private _lastTokenEndIndex;
    private readonly _tokenTypeOverrides;
    constructor(emitBinaryTokens: boolean, lineText: string, tokenTypeOverrides: TokenTypeMatcher[], balancedBracketSelectors: BalancedBracketSelectors | null);
    produce(stack: StateStack, endIndex: number): void;
    produceFromScopes(scopesList: AttributedScopeStack, endIndex: number): void;
    getResult(stack: StateStack, lineLength: number): IToken[];
    getBinaryResult(stack: StateStack, lineLength: number): Uint32Array;
}
export {};
