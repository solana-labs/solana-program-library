import type { Application } from "../application";
import { ProjectReflection, ReflectionVariant, TypeKindMap } from "../models/index";
import type { Logger } from "../utils/loggers";
import type { JSONOutput } from "./index";
export interface DeserializerComponent {
    priority: number;
    supports(model: unknown, obj: unknown): boolean;
    fromObject(model: unknown, obj: unknown): void;
}
export interface Deserializable<T> {
    fromObject(d: Deserializer, o: T): void;
}
export declare class Deserializer {
    private app;
    private deferred;
    private deserializers;
    private activeReflection;
    constructor(app: Application);
    get logger(): Logger;
    reflectionBuilders: {
        [K in keyof ReflectionVariant]: (parent: NonNullable<ReflectionVariant[K]["parent"]>, obj: JSONOutput.ModelToObject<ReflectionVariant[K]>) => ReflectionVariant[K];
    };
    typeBuilders: {
        [K in keyof TypeKindMap]: (obj: JSONOutput.ModelToObject<TypeKindMap[K]>, de: Deserializer) => TypeKindMap[K];
    };
    oldIdToNewId: Record<number, number | undefined>;
    project: ProjectReflection | undefined;
    addDeserializer(de: DeserializerComponent): void;
    /**
     * Revive a single project into the structure it was originally created with.
     * This is generally not appropriate for merging multiple projects since projects may
     * contain reflections in their root, not inside a module.
     */
    reviveProject(projectObj: JSONOutput.ProjectReflection, name?: string): ProjectReflection;
    reviveProjects(name: string, projects: readonly JSONOutput.ProjectReflection[]): ProjectReflection;
    revive<T, U extends Deserializable<T>>(source: NonNullable<T>, creator: (obj: T) => U): U;
    revive<T, U extends Deserializable<T>>(source: T | undefined, creator: (obj: T) => U): U | undefined;
    reviveMany<T, U extends Deserializable<T>>(sourceArray: T[], creator: (obj: T) => U): U[];
    reviveMany<T, U extends Deserializable<T>>(sourceArray: T[] | undefined, creator: (obj: T) => U): U[] | undefined;
    reviveType<T extends JSONOutput.SomeType>(obj: T): TypeKindMap[T["type"]];
    reviveType<T extends JSONOutput.SomeType>(obj: T | undefined): TypeKindMap[T["type"]] | undefined;
    constructReflection<T extends JSONOutput.SomeReflection>(obj: T): ReflectionVariant[T["variant"]];
    constructType<T extends JSONOutput.SomeType>(obj: T): TypeKindMap[T["type"]];
    fromObject<T>(receiver: {
        fromObject(d: Deserializer, o: T): void;
    }, obj: T): void;
    /**
     * Defers work until the initial pass of serialization has been completed.
     * This can be used to set up references which cannot be immediately restored.
     *
     * May only be called when deserializing.
     */
    defer(cb: (project: ProjectReflection) => void): void;
}
