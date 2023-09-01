import type { LineAndCharacter, SourceFileLike } from "typescript";
export declare class MinimalSourceFile implements SourceFileLike {
    readonly text: string;
    readonly fileName: string;
    constructor(text: string, fileName: string);
    getLineAndCharacterOfPosition(pos: number): LineAndCharacter;
}
