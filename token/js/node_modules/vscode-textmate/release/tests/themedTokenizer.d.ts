import { IGrammar } from '../main';
export interface IThemedToken {
    content: string;
    color: string;
}
export declare function tokenizeWithTheme(colorMap: string[], fileContents: string, grammar: IGrammar): IThemedToken[];
