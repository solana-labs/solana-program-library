import type { Deserializer } from "../../serialization/deserializer";
import type { SourceReference as JSONSourceReference } from "../../serialization/schema";
/**
 * Represents references of reflections to their defining source files.
 *
 * @see {@link DeclarationReflection.sources}
 */
export declare class SourceReference {
    /**
     * The filename of the source file.
     */
    fileName: string;
    /**
     * The absolute filename of the source file.
     */
    fullFileName: string;
    /**
     * The number of the line that emitted the declaration.
     */
    line: number;
    /**
     * The index of the character that emitted the declaration.
     */
    character: number;
    /**
     * URL for displaying the source file.
     */
    url?: string;
    constructor(fileName: string, line: number, character: number);
    toObject(): JSONSourceReference;
    fromObject(_de: Deserializer, obj: JSONSourceReference): void;
}
