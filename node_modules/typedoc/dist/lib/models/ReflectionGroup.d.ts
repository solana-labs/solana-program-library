import { ReflectionCategory } from "./ReflectionCategory";
import type { DeclarationReflection } from ".";
import type { Serializer, JSONOutput, Deserializer } from "../serialization";
/**
 * A group of reflections. All reflections in a group are of the same kind.
 *
 * Reflection groups are created by the ´GroupHandler´ in the resolving phase
 * of the dispatcher. The main purpose of groups is to be able to more easily
 * render human readable children lists in templates.
 */
export declare class ReflectionGroup {
    /**
     * The title, a string representation of the typescript kind, of this group.
     */
    title: string;
    /**
     * All reflections of this group.
     */
    children: DeclarationReflection[];
    /**
     * Categories contained within this group.
     */
    categories?: ReflectionCategory[];
    /**
     * Create a new ReflectionGroup instance.
     *
     * @param title The title of this group.
     */
    constructor(title: string);
    /**
     * Do all children of this group have a separate document?
     */
    allChildrenHaveOwnDocument(): boolean;
    toObject(serializer: Serializer): JSONOutput.ReflectionGroup;
    fromObject(de: Deserializer, obj: JSONOutput.ReflectionGroup): void;
}
