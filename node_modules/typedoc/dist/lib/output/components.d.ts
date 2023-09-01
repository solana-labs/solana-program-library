import { Component, AbstractComponent } from "../utils/component";
import type { ProjectReflection, Reflection } from "../models/reflections/index";
import type { Renderer } from "./renderer";
import { RendererEvent, PageEvent } from "./events";
export { Component };
export declare abstract class RendererComponent extends AbstractComponent<Renderer> {
}
/**
 * A plugin for the renderer that reads the current render context.
 */
export declare abstract class ContextAwareRendererComponent extends RendererComponent {
    /**
     * The project that is currently processed.
     */
    protected project?: ProjectReflection;
    /**
     * The reflection that is currently processed.
     */
    protected page?: PageEvent<Reflection>;
    /**
     * The url of the document that is being currently generated.
     * Set when a page begins rendering.
     */
    private location;
    /**
     * Regular expression to test if a string looks like an external url.
     */
    protected urlPrefix: RegExp;
    /**
     * Create a new ContextAwareRendererPlugin instance.
     *
     * @param renderer  The renderer this plugin should be attached to.
     */
    protected initialize(): void;
    /**
     * Transform the given absolute path into a relative path.
     *
     * @param absolute  The absolute path to transform.
     * @returns A path relative to the document currently processed.
     */
    getRelativeUrl(absolute: string): string;
    /**
     * Triggered before the renderer starts rendering a project.
     *
     * @param event  An event object describing the current render operation.
     */
    protected onBeginRenderer(event: RendererEvent): void;
    /**
     * Triggered before a document will be rendered.
     *
     * @param page  An event object describing the current render operation.
     */
    protected onBeginPage(page: PageEvent<Reflection>): void;
}
