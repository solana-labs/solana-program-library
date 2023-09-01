import type { LineTokens, StateStack } from './grammar';
import { OnigString } from '../onigLib';
import type { AttributedScopeStack, Grammar } from './grammar';
declare class TokenizeStringResult {
    readonly stack: StateStack;
    readonly stoppedEarly: boolean;
    constructor(stack: StateStack, stoppedEarly: boolean);
}
/**
 * Tokenize a string
 * @param grammar
 * @param lineText
 * @param isFirstLine
 * @param linePos
 * @param stack
 * @param lineTokens
 * @param checkWhileConditions
 * @param timeLimit Use `0` to indicate no time limit
 * @returns the StackElement or StackElement.TIME_LIMIT_REACHED if the time limit has been reached
 */
export declare function _tokenizeString(grammar: Grammar, lineText: OnigString, isFirstLine: boolean, linePos: number, stack: StateStack, lineTokens: LineTokens, checkWhileConditions: boolean, timeLimit: number): TokenizeStringResult;
export declare class LocalStackElement {
    readonly scopes: AttributedScopeStack;
    readonly endPos: number;
    constructor(scopes: AttributedScopeStack, endPos: number);
}
export {};
