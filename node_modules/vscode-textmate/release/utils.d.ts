import { IOnigCaptureIndex } from './onigLib';
export declare function clone<T>(something: T): T;
export declare function mergeObjects(target: any, ...sources: any[]): any;
export declare function basename(path: string): string;
export declare class RegexSource {
    static hasCaptures(regexSource: string | null): boolean;
    static replaceCaptures(regexSource: string, captureSource: string, captureIndices: IOnigCaptureIndex[]): string;
}
/**
 * A union of given const enum values.
*/
export declare type OrMask<T extends number> = number;
export declare function strcmp(a: string, b: string): number;
export declare function strArrCmp(a: string[] | null, b: string[] | null): number;
export declare function isValidHexColor(hex: string): boolean;
/**
 * Escapes regular expression characters in a given string
 */
export declare function escapeRegExpCharacters(value: string): string;
export declare class CachedFn<TKey, TValue> {
    private readonly fn;
    private readonly cache;
    constructor(fn: (key: TKey) => TValue);
    get(key: TKey): TValue;
}
export declare const performanceNow: () => number;
