import { RendererComponent } from "../components";
/**
 * A plugin that exports an index of the project to a javascript file.
 *
 * The resulting javascript file can be used to build a simple search function.
 */
export declare class JavascriptIndexPlugin extends RendererComponent {
    searchComments: boolean;
    /**
     * Create a new JavascriptIndexPlugin instance.
     */
    initialize(): void;
    /**
     * Triggered after a document has been rendered, just before it is written to disc.
     *
     * @param event  An event object describing the current render operation.
     */
    private onRendererBegin;
    private getCommentSearchText;
}
