import { OrMask } from "./utils";
export interface IOnigLib {
    createOnigScanner(sources: string[]): OnigScanner;
    createOnigString(str: string): OnigString;
}
export interface IOnigCaptureIndex {
    start: number;
    end: number;
    length: number;
}
export interface IOnigMatch {
    index: number;
    captureIndices: IOnigCaptureIndex[];
}
export declare const enum FindOption {
    None = 0,
    /**
     * equivalent of ONIG_OPTION_NOT_BEGIN_STRING: (str) isn't considered as begin of string (* fail \A)
     */
    NotBeginString = 1,
    /**
     * equivalent of ONIG_OPTION_NOT_END_STRING: (end) isn't considered as end of string (* fail \z, \Z)
     */
    NotEndString = 2,
    /**
     * equivalent of ONIG_OPTION_NOT_BEGIN_POSITION: (start) isn't considered as start position of search (* fail \G)
     */
    NotBeginPosition = 4,
    /**
     * used for debugging purposes.
     */
    DebugCall = 8
}
export interface OnigScanner {
    findNextMatchSync(string: string | OnigString, startPosition: number, options: OrMask<FindOption>): IOnigMatch | null;
    dispose?(): void;
}
export interface OnigString {
    readonly content: string;
    dispose?(): void;
}
export declare function disposeOnigString(str: OnigString): void;
