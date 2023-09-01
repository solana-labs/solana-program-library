import { IRawRule } from '../rawGrammar';
import { ScopeName } from '../theme';
import { IGrammarRepository } from './grammar';
export declare type AbsoluteRuleReference = TopLevelRuleReference | TopLevelRepositoryRuleReference;
/**
 * References the top level rule of a grammar with the given scope name.
*/
export declare class TopLevelRuleReference {
    readonly scopeName: ScopeName;
    constructor(scopeName: ScopeName);
    toKey(): string;
}
/**
 * References a rule of a grammar in the top level repository section with the given name.
*/
export declare class TopLevelRepositoryRuleReference {
    readonly scopeName: ScopeName;
    readonly ruleName: string;
    constructor(scopeName: ScopeName, ruleName: string);
    toKey(): string;
}
export declare class ExternalReferenceCollector {
    private readonly _references;
    private readonly _seenReferenceKeys;
    get references(): readonly AbsoluteRuleReference[];
    readonly visitedRule: Set<IRawRule>;
    add(reference: AbsoluteRuleReference): void;
}
export declare class ScopeDependencyProcessor {
    readonly repo: IGrammarRepository;
    readonly initialScopeName: ScopeName;
    readonly seenFullScopeRequests: Set<string>;
    readonly seenPartialScopeRequests: Set<string>;
    Q: AbsoluteRuleReference[];
    constructor(repo: IGrammarRepository, initialScopeName: ScopeName);
    processQueue(): void;
}
export declare type IncludeReference = BaseReference | SelfReference | RelativeReference | TopLevelReference | TopLevelRepositoryReference;
export declare const enum IncludeReferenceKind {
    Base = 0,
    Self = 1,
    RelativeReference = 2,
    TopLevelReference = 3,
    TopLevelRepositoryReference = 4
}
export declare class BaseReference {
    readonly kind = IncludeReferenceKind.Base;
}
export declare class SelfReference {
    readonly kind = IncludeReferenceKind.Self;
}
export declare class RelativeReference {
    readonly ruleName: string;
    readonly kind = IncludeReferenceKind.RelativeReference;
    constructor(ruleName: string);
}
export declare class TopLevelReference {
    readonly scopeName: ScopeName;
    readonly kind = IncludeReferenceKind.TopLevelReference;
    constructor(scopeName: ScopeName);
}
export declare class TopLevelRepositoryReference {
    readonly scopeName: ScopeName;
    readonly ruleName: string;
    readonly kind = IncludeReferenceKind.TopLevelRepositoryReference;
    constructor(scopeName: ScopeName, ruleName: string);
}
export declare function parseInclude(include: string): IncludeReference;
