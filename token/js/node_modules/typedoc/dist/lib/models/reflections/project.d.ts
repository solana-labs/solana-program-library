import { Reflection } from "./abstract";
import { ContainerReflection } from "./container";
import type * as ts from "typescript";
import { ReflectionKind } from "./kind";
import { CommentDisplayPart } from "../comments";
import { ReflectionSymbolId } from "./ReflectionSymbolId";
import type { Serializer } from "../../serialization/serializer";
import type { Deserializer, JSONOutput } from "../../serialization/index";
/**
 * A reflection that represents the root of the project.
 *
 * The project reflection acts as a global index, one may receive all reflections
 * and source files of the processed project through this reflection.
 */
export declare class ProjectReflection extends ContainerReflection {
    readonly variant = "project";
    private symbolToReflectionIdMap;
    private reflectionIdToSymbolIdMap;
    private reflectionIdToSymbolMap;
    private referenceGraph?;
    private reflectionChildren;
    /**
     * A list of all reflections within the project. DO NOT MUTATE THIS OBJECT.
     * All mutation should be done via {@link registerReflection} and {@link removeReflection}
     * to ensure that links to reflections remain valid.
     *
     * This may be replaced with a `Map<number, Reflection>` someday.
     */
    reflections: {
        [id: number]: Reflection;
    };
    /**
     * The name of the package that this reflection documents according to package.json.
     */
    packageName?: string;
    /**
     * The version of the package that this reflection documents according to package.json.
     */
    packageVersion?: string;
    /**
     * The contents of the readme.md file of the project when found.
     */
    readme?: CommentDisplayPart[];
    constructor(name: string);
    /**
     * Return whether this reflection is the root / project reflection.
     */
    isProject(): this is ProjectReflection;
    /**
     * Return a list of all reflections in this project of a certain kind.
     *
     * @param kind  The desired kind of reflection.
     * @returns     An array containing all reflections with the desired kind.
     */
    getReflectionsByKind(kind: ReflectionKind): Reflection[];
    /**
     * Registers the given reflection so that it can be quickly looked up by helper methods.
     * Should be called for *every* reflection added to the project.
     */
    registerReflection(reflection: Reflection, symbol?: ts.Symbol): void;
    /**
     * Removes a reflection from the documentation. Can be used by plugins to filter reflections
     * out of the generated documentation. Has no effect if the reflection is not present in the
     * project.
     */
    removeReflection(reflection: Reflection): void;
    /**
     * Remove a reflection without updating the parent reflection to remove references to the removed reflection.
     */
    private _removeReflection;
    /**
     * Gets the reflection registered for the given reflection ID, or undefined if it is not present
     * in the project.
     */
    getReflectionById(id: number): Reflection | undefined;
    /**
     * Gets the reflection associated with the given symbol, if it exists.
     * @internal
     */
    getReflectionFromSymbol(symbol: ts.Symbol): Reflection | undefined;
    /**
     * Gets the reflection associated with the given symbol id, if it exists.
     * @internal
     */
    getReflectionFromSymbolId(symbolId: ReflectionSymbolId): Reflection | undefined;
    /** @internal */
    getSymbolIdFromReflection(reflection: Reflection): ReflectionSymbolId | undefined;
    /** @internal */
    registerSymbolId(reflection: Reflection, id: ReflectionSymbolId): void;
    /**
     * THIS MAY NOT BE USED AFTER CONVERSION HAS FINISHED.
     * @internal
     */
    getSymbolFromReflection(reflection: Reflection): ts.Symbol | undefined;
    private getReferenceGraph;
    toObject(serializer: Serializer): JSONOutput.ProjectReflection;
    fromObject(de: Deserializer, obj: JSONOutput.ProjectReflection): void;
}
