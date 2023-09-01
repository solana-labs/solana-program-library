import ts from "typescript";
import type { JSONOutput, Serializer } from "../../serialization/index";
/**
 * See {@link ReflectionSymbolId}
 */
export type ReflectionSymbolIdString = string & {
    readonly __reflectionSymbolId: unique symbol;
};
/**
 * This exists so that TypeDoc can store a unique identifier for a `ts.Symbol` without
 * keeping a reference to the `ts.Symbol` itself. This identifier should be stable across
 * runs so long as the symbol is exported from the same file.
 */
export declare class ReflectionSymbolId {
    readonly fileName: string;
    readonly qualifiedName: string;
    /**
     * Note: This is **not** serialized. It exists for sorting by declaration order, but
     * should not be needed when deserializing from JSON.
     */
    pos: number;
    constructor(symbol: ts.Symbol, declaration?: ts.Declaration);
    constructor(json: JSONOutput.ReflectionSymbolId);
    getStableKey(): ReflectionSymbolIdString;
    toObject(serializer: Serializer): {
        sourceFileName: string;
        qualifiedName: string;
    };
}
