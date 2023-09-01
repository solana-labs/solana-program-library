import { IOnigLib } from '../onigLib';
import { RegistryOptions } from '../main';
import { IRawGrammar } from '../rawGrammar';
export interface ILanguageRegistration {
    id: string;
    extensions: string[];
    filenames: string[];
}
export interface IGrammarRegistration {
    language: string;
    scopeName: string;
    path: string;
    embeddedLanguages: {
        [scopeName: string]: string;
    };
    grammar?: Promise<IRawGrammar>;
}
export declare class Resolver implements RegistryOptions {
    readonly language2id: {
        [languages: string]: number;
    };
    private _lastLanguageId;
    private _id2language;
    private readonly _grammars;
    private readonly _languages;
    readonly onigLib: Promise<IOnigLib>;
    constructor(grammars: IGrammarRegistration[], languages: ILanguageRegistration[], onigLibPromise: Promise<IOnigLib>);
    findLanguageByExtension(fileExtension: string): string | null;
    findLanguageByFilename(filename: string): string | null;
    findScopeByFilename(filename: string): string | null;
    findGrammarByLanguage(language: string): IGrammarRegistration;
    loadGrammar(scopeName: string): Promise<IRawGrammar | null>;
}
