import { ContextAwareRendererComponent } from "../components";
import { RendererEvent, MarkdownEvent, PageEvent } from "../events";
import type { Theme } from "shiki";
/**
 * Implements markdown and relativeURL helpers for templates.
 * @internal
 */
export declare class MarkedPlugin extends ContextAwareRendererComponent {
    includeSource: string;
    mediaSource: string;
    lightTheme: Theme;
    darkTheme: Theme;
    /**
     * The path referenced files are located in.
     */
    private includes?;
    /**
     * Path to the output media directory.
     */
    private mediaDirectory?;
    /**
     * The pattern used to find references in markdown.
     */
    private includePattern;
    /**
     * The pattern used to find media links.
     */
    private mediaPattern;
    private sources?;
    private outputFileName?;
    /**
     * Create a new MarkedPlugin instance.
     */
    initialize(): void;
    /**
     * Highlight the syntax of the given text using HighlightJS.
     *
     * @param text  The text that should be highlighted.
     * @param lang  The language that should be used to highlight the string.
     * @return A html string with syntax highlighting.
     */
    getHighlighted(text: string, lang?: string): string;
    /**
     * Parse the given markdown string and return the resulting html.
     *
     * @param text  The markdown string that should be parsed.
     * @returns The resulting html string.
     */
    parseMarkdown(text: string, page: PageEvent<any>): string;
    /**
     * Triggered before the renderer starts rendering a project.
     *
     * @param event  An event object describing the current render operation.
     */
    protected onBeginRenderer(event: RendererEvent): void;
    /**
     * Creates an object with options that are passed to the markdown parser.
     *
     * @returns The options object for the markdown parser.
     */
    private createMarkedOptions;
    /**
     * Triggered when {@link MarkedPlugin} parses a markdown string.
     *
     * @param event
     */
    onParseMarkdown(event: MarkdownEvent): void;
}
