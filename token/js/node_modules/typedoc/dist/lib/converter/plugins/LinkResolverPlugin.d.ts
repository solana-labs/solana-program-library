import { ConverterComponent } from "../components";
import type { Context } from "../../converter";
import { ValidationOptions } from "../../utils";
import { ProjectReflection } from "../../models";
/**
 * A plugin that resolves `{@link Foo}` tags.
 */
export declare class LinkResolverPlugin extends ConverterComponent {
    validation: ValidationOptions;
    initialize(): void;
    onResolve(context: Context): void;
    resolveLinks(project: ProjectReflection): void;
}
