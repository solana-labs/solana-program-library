import type { SomeType } from "../types";
import { Reflection, TraverseCallback } from "./abstract";
import type { DeclarationReflection } from "./declaration";
import type { Serializer, JSONOutput, Deserializer } from "../../serialization";
import type { SignatureReflection } from "./signature";
/**
 * Modifier flags for type parameters, added in TS 4.7
 * @enum
 */
export declare const VarianceModifier: {
    readonly in: "in";
    readonly out: "out";
    readonly inOut: "in out";
};
export type VarianceModifier = (typeof VarianceModifier)[keyof typeof VarianceModifier];
export declare class TypeParameterReflection extends Reflection {
    readonly variant = "typeParam";
    parent?: DeclarationReflection | SignatureReflection;
    type?: SomeType;
    default?: SomeType;
    varianceModifier?: VarianceModifier;
    constructor(name: string, parent: Reflection, varianceModifier: VarianceModifier | undefined);
    toObject(serializer: Serializer): JSONOutput.TypeParameterReflection;
    fromObject(de: Deserializer, obj: JSONOutput.TypeParameterReflection): void;
    traverse(_callback: TraverseCallback): void;
}
