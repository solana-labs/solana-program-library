import { Theme } from "shiki";
export declare function loadHighlighter(lightTheme: Theme, darkTheme: Theme): Promise<void>;
export declare function isSupportedLanguage(lang: string): boolean;
export declare function getSupportedLanguages(): string[];
export declare function highlight(code: string, lang: string): string;
export declare function getStyles(): string;
