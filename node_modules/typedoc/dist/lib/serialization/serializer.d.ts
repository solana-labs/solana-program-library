import { EventDispatcher } from "../utils";
import type { ProjectReflection } from "../models";
import type { ModelToObject } from "./schema";
import type { SerializerComponent } from "./components";
export declare class Serializer extends EventDispatcher {
    /**
     * Triggered when the {@link Serializer} begins transforming a project.
     * @event EVENT_BEGIN
     */
    static EVENT_BEGIN: string;
    /**
     * Triggered when the {@link Serializer} has finished transforming a project.
     * @event EVENT_END
     */
    static EVENT_END: string;
    private serializers;
    /**
     * Only set when serializing.
     */
    projectRoot: string;
    addSerializer(serializer: SerializerComponent<any>): void;
    toObject<T extends {
        toObject(serializer: Serializer): ModelToObject<T>;
    }>(value: T): ModelToObject<T>;
    toObject<T extends {
        toObject(serializer: Serializer): ModelToObject<T>;
    }>(value: T | undefined): ModelToObject<T> | undefined;
    toObjectsOptional<T extends {
        toObject(serializer: Serializer): ModelToObject<T>;
    }>(value: T[] | undefined): ModelToObject<T>[] | undefined;
    /**
     * Same as toObject but emits {@link Serializer.EVENT_BEGIN} and {@link Serializer.EVENT_END} events.
     * @param value
     * @param eventData Partial information to set in the event
     */
    projectToObject(value: ProjectReflection, projectRoot: string): ModelToObject<ProjectReflection>;
}
