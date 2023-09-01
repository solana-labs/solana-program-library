import { SomeType, ReferenceType } from "../types";
import { Reflection, TraverseCallback } from "./abstract";
import type { ParameterReflection } from "./parameter";
import type { TypeParameterReflection } from "./type-parameter";
import type { DeclarationReflection } from "./declaration";
import type { ReflectionKind } from "./kind";
import type { Serializer, JSONOutput, Deserializer } from "../../serialization";
import { SourceReference } from "../sources/file";
export declare class SignatureReflection extends Reflection {
    readonly variant = "signature";
    constructor(name: string, kind: SignatureReflection["kind"], parent: DeclarationReflection);
    kind: ReflectionKind.SetSignature | ReflectionKind.GetSignature | ReflectionKind.IndexSignature | ReflectionKind.CallSignature | ReflectionKind.ConstructorSignature;
    parent: DeclarationReflection;
    /**
     * A list of all source files that contributed to this reflection.
     */
    sources?: SourceReference[];
    parameters?: ParameterReflection[];
    typeParameters?: TypeParameterReflection[];
    type?: SomeType;
    /**
     * A type that points to the reflection that has been overwritten by this reflection.
     *
     * Applies to interface and class members.
     */
    overwrites?: ReferenceType;
    /**
     * A type that points to the reflection this reflection has been inherited from.
     *
     * Applies to interface and class members.
     */
    inheritedFrom?: ReferenceType;
    /**
     * A type that points to the reflection this reflection is the implementation of.
     *
     * Applies to class members.
     */
    implementationOf?: ReferenceType;
    traverse(callback: TraverseCallback): void;
    /**
     * Return a string representation of this reflection.
     */
    toString(): string;
    toObject(serializer: Serializer): JSONOutput.SignatureReflection;
    fromObject(de: Deserializer, obj: JSONOutput.SignatureReflection): void;
}
