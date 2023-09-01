export interface ILocation {
    readonly filename: string | null;
    readonly line: number;
    readonly char: number;
}
export declare function parseJSON(source: string, filename: string | null, withMetadata: boolean): any;
