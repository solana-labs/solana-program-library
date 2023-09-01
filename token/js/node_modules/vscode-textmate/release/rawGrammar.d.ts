import { RuleId } from "./rule";
export interface ILocation {
    readonly filename: string;
    readonly line: number;
    readonly char: number;
}
export interface ILocatable {
    readonly $vscodeTextmateLocation?: ILocation;
}
export interface IRawGrammar extends ILocatable {
    repository: IRawRepository;
    readonly scopeName: string;
    readonly patterns: IRawRule[];
    readonly injections?: {
        [expression: string]: IRawRule;
    };
    readonly injectionSelector?: string;
    readonly fileTypes?: string[];
    readonly name?: string;
    readonly firstLineMatch?: string;
}
export interface IRawRepositoryMap {
    [name: string]: IRawRule;
    $self: IRawRule;
    $base: IRawRule;
}
export declare type IRawRepository = IRawRepositoryMap & ILocatable;
export interface IRawRule extends ILocatable {
    id?: RuleId;
    readonly include?: string;
    readonly name?: string;
    readonly contentName?: string;
    readonly match?: string;
    readonly captures?: IRawCaptures;
    readonly begin?: string;
    readonly beginCaptures?: IRawCaptures;
    readonly end?: string;
    readonly endCaptures?: IRawCaptures;
    readonly while?: string;
    readonly whileCaptures?: IRawCaptures;
    readonly patterns?: IRawRule[];
    readonly repository?: IRawRepository;
    readonly applyEndPatternLast?: boolean;
}
export interface IRawCapturesMap {
    [captureId: string]: IRawRule;
}
export declare type IRawCaptures = IRawCapturesMap & ILocatable;
