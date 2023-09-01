import { RendererComponent } from "../components";
/**
 * A plugin that copies the subdirectory ´assets´ from the current themes
 * source folder to the output directory.
 */
export declare class AssetsPlugin extends RendererComponent {
    /** @internal */
    customCss: string;
    /**
     * Create a new AssetsPlugin instance.
     */
    initialize(): void;
    /**
     * Triggered before the renderer starts rendering a project.
     *
     * @param event  An event object describing the current render operation.
     */
    private onRenderEnd;
}
