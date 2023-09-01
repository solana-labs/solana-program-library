import { OrMask } from './utils';
import { IOnigLib, IOnigCaptureIndex, FindOption, OnigString } from './onigLib';
import { ILocation, IRawGrammar, IRawRepository, IRawRule } from './rawGrammar';
declare const ruleIdSymbol: unique symbol;
export declare type RuleId = {
    __brand: typeof ruleIdSymbol;
};
export declare const endRuleId = -1;
export declare const whileRuleId = -2;
export declare function ruleIdFromNumber(id: number): RuleId;
export declare function ruleIdToNumber(id: RuleId): number;
export interface IRuleRegistry {
    getRule(ruleId: RuleId): Rule;
    registerRule<T extends Rule>(factory: (id: RuleId) => T): T;
}
export interface IGrammarRegistry {
    getExternalGrammar(scopeName: string, repository: IRawRepository): IRawGrammar | null | undefined;
}
export interface IRuleFactoryHelper extends IRuleRegistry, IGrammarRegistry {
}
export declare abstract class Rule {
    readonly $location: ILocation | undefined;
    readonly id: RuleId;
    private readonly _nameIsCapturing;
    private readonly _name;
    private readonly _contentNameIsCapturing;
    private readonly _contentName;
    constructor($location: ILocation | undefined, id: RuleId, name: string | null | undefined, contentName: string | null | undefined);
    abstract dispose(): void;
    get debugName(): string;
    getName(lineText: string | null, captureIndices: IOnigCaptureIndex[] | null): string | null;
    getContentName(lineText: string, captureIndices: IOnigCaptureIndex[]): string | null;
    abstract collectPatterns(grammar: IRuleRegistry, out: RegExpSourceList): void;
    abstract compile(grammar: IRuleRegistry & IOnigLib, endRegexSource: string | null): CompiledRule;
    abstract compileAG(grammar: IRuleRegistry & IOnigLib, endRegexSource: string | null, allowA: boolean, allowG: boolean): CompiledRule;
}
export interface ICompilePatternsResult {
    readonly patterns: RuleId[];
    readonly hasMissingPatterns: boolean;
}
export declare class CaptureRule extends Rule {
    readonly retokenizeCapturedWithRuleId: RuleId | 0;
    constructor($location: ILocation | undefined, id: RuleId, name: string | null | undefined, contentName: string | null | undefined, retokenizeCapturedWithRuleId: RuleId | 0);
    dispose(): void;
    collectPatterns(grammar: IRuleRegistry, out: RegExpSourceList): void;
    compile(grammar: IRuleRegistry & IOnigLib, endRegexSource: string): CompiledRule;
    compileAG(grammar: IRuleRegistry & IOnigLib, endRegexSource: string, allowA: boolean, allowG: boolean): CompiledRule;
}
export declare class MatchRule extends Rule {
    private readonly _match;
    readonly captures: (CaptureRule | null)[];
    private _cachedCompiledPatterns;
    constructor($location: ILocation | undefined, id: RuleId, name: string | undefined, match: string, captures: (CaptureRule | null)[]);
    dispose(): void;
    get debugMatchRegExp(): string;
    collectPatterns(grammar: IRuleRegistry, out: RegExpSourceList): void;
    compile(grammar: IRuleRegistry & IOnigLib, endRegexSource: string): CompiledRule;
    compileAG(grammar: IRuleRegistry & IOnigLib, endRegexSource: string, allowA: boolean, allowG: boolean): CompiledRule;
    private _getCachedCompiledPatterns;
}
export declare class IncludeOnlyRule extends Rule {
    readonly hasMissingPatterns: boolean;
    readonly patterns: RuleId[];
    private _cachedCompiledPatterns;
    constructor($location: ILocation | undefined, id: RuleId, name: string | null | undefined, contentName: string | null | undefined, patterns: ICompilePatternsResult);
    dispose(): void;
    collectPatterns(grammar: IRuleRegistry, out: RegExpSourceList): void;
    compile(grammar: IRuleRegistry & IOnigLib, endRegexSource: string): CompiledRule;
    compileAG(grammar: IRuleRegistry & IOnigLib, endRegexSource: string, allowA: boolean, allowG: boolean): CompiledRule;
    private _getCachedCompiledPatterns;
}
export declare class BeginEndRule extends Rule {
    private readonly _begin;
    readonly beginCaptures: (CaptureRule | null)[];
    private readonly _end;
    readonly endHasBackReferences: boolean;
    readonly endCaptures: (CaptureRule | null)[];
    readonly applyEndPatternLast: boolean;
    readonly hasMissingPatterns: boolean;
    readonly patterns: RuleId[];
    private _cachedCompiledPatterns;
    constructor($location: ILocation | undefined, id: RuleId, name: string | null | undefined, contentName: string | null | undefined, begin: string, beginCaptures: (CaptureRule | null)[], end: string | undefined, endCaptures: (CaptureRule | null)[], applyEndPatternLast: boolean | undefined, patterns: ICompilePatternsResult);
    dispose(): void;
    get debugBeginRegExp(): string;
    get debugEndRegExp(): string;
    getEndWithResolvedBackReferences(lineText: string, captureIndices: IOnigCaptureIndex[]): string;
    collectPatterns(grammar: IRuleRegistry, out: RegExpSourceList): void;
    compile(grammar: IRuleRegistry & IOnigLib, endRegexSource: string): CompiledRule;
    compileAG(grammar: IRuleRegistry & IOnigLib, endRegexSource: string, allowA: boolean, allowG: boolean): CompiledRule;
    private _getCachedCompiledPatterns;
}
export declare class BeginWhileRule extends Rule {
    private readonly _begin;
    readonly beginCaptures: (CaptureRule | null)[];
    readonly whileCaptures: (CaptureRule | null)[];
    private readonly _while;
    readonly whileHasBackReferences: boolean;
    readonly hasMissingPatterns: boolean;
    readonly patterns: RuleId[];
    private _cachedCompiledPatterns;
    private _cachedCompiledWhilePatterns;
    constructor($location: ILocation | undefined, id: RuleId, name: string | null | undefined, contentName: string | null | undefined, begin: string, beginCaptures: (CaptureRule | null)[], _while: string, whileCaptures: (CaptureRule | null)[], patterns: ICompilePatternsResult);
    dispose(): void;
    get debugBeginRegExp(): string;
    get debugWhileRegExp(): string;
    getWhileWithResolvedBackReferences(lineText: string, captureIndices: IOnigCaptureIndex[]): string;
    collectPatterns(grammar: IRuleRegistry, out: RegExpSourceList): void;
    compile(grammar: IRuleRegistry & IOnigLib, endRegexSource: string): CompiledRule;
    compileAG(grammar: IRuleRegistry & IOnigLib, endRegexSource: string, allowA: boolean, allowG: boolean): CompiledRule;
    private _getCachedCompiledPatterns;
    compileWhile(grammar: IRuleRegistry & IOnigLib, endRegexSource: string | null): CompiledRule<RuleId | typeof whileRuleId>;
    compileWhileAG(grammar: IRuleRegistry & IOnigLib, endRegexSource: string | null, allowA: boolean, allowG: boolean): CompiledRule<RuleId | typeof whileRuleId>;
    private _getCachedCompiledWhilePatterns;
}
export declare class RuleFactory {
    static createCaptureRule(helper: IRuleFactoryHelper, $location: ILocation | undefined, name: string | null | undefined, contentName: string | null | undefined, retokenizeCapturedWithRuleId: RuleId | 0): CaptureRule;
    static getCompiledRuleId(desc: IRawRule, helper: IRuleFactoryHelper, repository: IRawRepository): RuleId;
    private static _compileCaptures;
    private static _compilePatterns;
}
export declare class RegExpSource<TRuleId = RuleId | typeof endRuleId> {
    source: string;
    readonly ruleId: TRuleId;
    hasAnchor: boolean;
    readonly hasBackReferences: boolean;
    private _anchorCache;
    constructor(regExpSource: string, ruleId: TRuleId);
    clone(): RegExpSource<TRuleId>;
    setSource(newSource: string): void;
    resolveBackReferences(lineText: string, captureIndices: IOnigCaptureIndex[]): string;
    private _buildAnchorCache;
    resolveAnchors(allowA: boolean, allowG: boolean): string;
}
export declare class RegExpSourceList<TRuleId = RuleId | typeof endRuleId> {
    private readonly _items;
    private _hasAnchors;
    private _cached;
    private _anchorCache;
    constructor();
    dispose(): void;
    private _disposeCaches;
    push(item: RegExpSource<TRuleId>): void;
    unshift(item: RegExpSource<TRuleId>): void;
    length(): number;
    setSource(index: number, newSource: string): void;
    compile(onigLib: IOnigLib): CompiledRule<TRuleId>;
    compileAG(onigLib: IOnigLib, allowA: boolean, allowG: boolean): CompiledRule<TRuleId>;
    private _resolveAnchors;
}
export declare class CompiledRule<TRuleId = RuleId | typeof endRuleId> {
    private readonly regExps;
    private readonly rules;
    private readonly scanner;
    constructor(onigLib: IOnigLib, regExps: string[], rules: TRuleId[]);
    dispose(): void;
    toString(): string;
    findNextMatchSync(string: string | OnigString, startPosition: number, options: OrMask<FindOption>): IFindNextMatchResult<TRuleId> | null;
}
export interface IFindNextMatchResult<TRuleId = RuleId | typeof endRuleId> {
    ruleId: TRuleId;
    captureIndices: IOnigCaptureIndex[];
}
export {};
