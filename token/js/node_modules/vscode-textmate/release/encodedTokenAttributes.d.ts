import { FontStyle } from "./theme";
export declare type EncodedTokenAttributes = number;
export declare namespace EncodedTokenAttributes {
    function toBinaryStr(encodedTokenAttributes: EncodedTokenAttributes): string;
    function print(encodedTokenAttributes: EncodedTokenAttributes): void;
    function getLanguageId(encodedTokenAttributes: EncodedTokenAttributes): number;
    function getTokenType(encodedTokenAttributes: EncodedTokenAttributes): StandardTokenType;
    function containsBalancedBrackets(encodedTokenAttributes: EncodedTokenAttributes): boolean;
    function getFontStyle(encodedTokenAttributes: EncodedTokenAttributes): number;
    function getForeground(encodedTokenAttributes: EncodedTokenAttributes): number;
    function getBackground(encodedTokenAttributes: EncodedTokenAttributes): number;
    /**
     * Updates the fields in `metadata`.
     * A value of `0`, `NotSet` or `null` indicates that the corresponding field should be left as is.
     */
    function set(encodedTokenAttributes: EncodedTokenAttributes, languageId: number, tokenType: OptionalStandardTokenType, containsBalancedBrackets: boolean | null, fontStyle: FontStyle, foreground: number, background: number): number;
}
export declare const enum StandardTokenType {
    Other = 0,
    Comment = 1,
    String = 2,
    RegEx = 3
}
export declare function toOptionalTokenType(standardType: StandardTokenType): OptionalStandardTokenType;
export declare const enum OptionalStandardTokenType {
    Other = 0,
    Comment = 1,
    String = 2,
    RegEx = 3,
    NotSet = 8
}
