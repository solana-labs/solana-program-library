import { Reflection, TraverseCallback } from "./abstract";
import { ReflectionCategory } from "../ReflectionCategory";
import { ReflectionGroup } from "../ReflectionGroup";
import type { ReflectionKind } from "./kind";
import type { Serializer, JSONOutput, Deserializer } from "../../serialization";
import type { DeclarationReflection } from "./declaration";
export declare abstract class ContainerReflection extends Reflection {
    /**
     * The children of this reflection.
     */
    children?: DeclarationReflection[];
    /**
     * All children grouped by their kind.
     */
    groups?: ReflectionGroup[];
    /**
     * All children grouped by their category.
     */
    categories?: ReflectionCategory[];
    /**
     * Return a list of all children of a certain kind.
     *
     * @param kind  The desired kind of children.
     * @returns     An array containing all children with the desired kind.
     */
    getChildrenByKind(kind: ReflectionKind): DeclarationReflection[];
    traverse(callback: TraverseCallback): void;
    toObject(serializer: Serializer): JSONOutput.ContainerReflection;
    fromObject(de: Deserializer, obj: JSONOutput.ContainerReflection): void;
}
