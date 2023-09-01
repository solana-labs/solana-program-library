import type { Application } from "../application";
import type { Theme } from "./theme";
import { RendererEvent } from "./events";
import type { ProjectReflection } from "../models/reflections/project";
import { RendererComponent } from "./components";
import { ChildableComponent } from "../utils/component";
import { EventHooks } from "../utils";
import type { Theme as ShikiTheme } from "shiki";
import type { JsxElement } from "../utils/jsx.elements";
import type { DefaultThemeRenderContext } from "./themes/default/DefaultThemeRenderContext";
/**
 * Describes the hooks available to inject output in the default theme.
 * If the available hooks don't let you put something where you'd like, please open an issue!
 */
export interface RendererHooks {
    /**
     * Applied immediately after the opening `<head>` tag.
     */
    "head.begin": [DefaultThemeRenderContext];
    /**
     * Applied immediately before the closing `</head>` tag.
     */
    "head.end": [DefaultThemeRenderContext];
    /**
     * Applied immediately after the opening `<body>` tag.
     */
    "body.begin": [DefaultThemeRenderContext];
    /**
     * Applied immediately before the closing `</body>` tag.
     */
    "body.end": [DefaultThemeRenderContext];
    /**
     * Applied immediately before the main template.
     */
    "content.begin": [DefaultThemeRenderContext];
    /**
     * Applied immediately after the main template.
     */
    "content.end": [DefaultThemeRenderContext];
    /**
     * Applied immediately before calling `context.sidebar`.
     */
    "sidebar.begin": [DefaultThemeRenderContext];
    /**
     * Applied immediately after calling `context.sidebar`.
     */
    "sidebar.end": [DefaultThemeRenderContext];
    /**
     * Applied immediately before calling `context.pageSidebar`.
     */
    "pageSidebar.begin": [DefaultThemeRenderContext];
    /**
     * Applied immediately after calling `context.pageSidebar`.
     */
    "pageSidebar.end": [DefaultThemeRenderContext];
}
/**
 * The renderer processes a {@link ProjectReflection} using a {@link Theme} instance and writes
 * the emitted html documents to a output directory. You can specify which theme should be used
 * using the `--theme <name>` command line argument.
 *
 * {@link Renderer} is a subclass of {@link EventDispatcher} and triggers a series of events while
 * a project is being processed. You can listen to these events to control the flow or manipulate
 * the output.
 *
 *  * {@link Renderer.EVENT_BEGIN}<br>
 *    Triggered before the renderer starts rendering a project. The listener receives
 *    an instance of {@link RendererEvent}. By calling {@link RendererEvent.preventDefault} the entire
 *    render process can be canceled.
 *
 *    * {@link Renderer.EVENT_BEGIN_PAGE}<br>
 *      Triggered before a document will be rendered. The listener receives an instance of
 *      {@link PageEvent}. By calling {@link PageEvent.preventDefault} the generation of the
 *      document can be canceled.
 *
 *    * {@link Renderer.EVENT_END_PAGE}<br>
 *      Triggered after a document has been rendered, just before it is written to disc. The
 *      listener receives an instance of {@link PageEvent}. When calling
 *      {@link PageEvent.preventDefault} the the document will not be saved to disc.
 *
 *  * {@link Renderer.EVENT_END}<br>
 *    Triggered after the renderer has written all documents. The listener receives
 *    an instance of {@link RendererEvent}.
 *
 * * {@link Renderer.EVENT_PREPARE_INDEX}<br>
 *    Triggered when the JavascriptIndexPlugin is preparing the search index. Listeners receive
 *    an instance of {@link IndexEvent}.
 */
export declare class Renderer extends ChildableComponent<Application, RendererComponent> {
    private themes;
    /** @event */
    static readonly EVENT_BEGIN_PAGE = "beginPage";
    /** @event */
    static readonly EVENT_END_PAGE = "endPage";
    /** @event */
    static readonly EVENT_BEGIN = "beginRender";
    /** @event */
    static readonly EVENT_END = "endRender";
    /** @event */
    static readonly EVENT_PREPARE_INDEX = "prepareIndex";
    /**
     * A list of async jobs which must be completed *before* rendering output.
     * They will be called after {@link RendererEvent.BEGIN} has fired, but before any files have been written.
     *
     * This may be used by plugins to register work that must be done to prepare output files. For example: asynchronously
     * transform markdown to HTML.
     *
     * Note: This array is cleared after calling the contained functions on each {@link Renderer.render} call.
     */
    preRenderAsyncJobs: Array<(output: RendererEvent) => Promise<void>>;
    /**
     * A list of async jobs which must be completed after rendering output files but before generation is considered successful.
     * These functions will be called after all documents have been written to the filesystem.
     *
     * This may be used by plugins to register work that must be done to finalize output files. For example: asynchronously
     * generating an image referenced in a render hook.
     *
     * Note: This array is cleared after calling the contained functions on each {@link Renderer.render} call.
     */
    postRenderAsyncJobs: Array<(output: RendererEvent) => Promise<void>>;
    /**
     * The theme that is used to render the documentation.
     */
    theme?: Theme;
    /**
     * Hooks which will be called when rendering pages.
     * Note:
     * - Hooks added during output will be discarded at the end of rendering.
     * - Hooks added during a page render will be discarded at the end of that page's render.
     *
     * See {@link RendererHooks} for a description of each available hook, and when it will be called.
     */
    hooks: EventHooks<RendererHooks, JsxElement>;
    /** @internal */
    themeName: string;
    /** @internal */
    cleanOutputDir: boolean;
    /** @internal */
    cname: string;
    /** @internal */
    githubPages: boolean;
    /** @internal */
    cacheBust: boolean;
    /** @internal */
    lightTheme: ShikiTheme;
    /** @internal */
    darkTheme: ShikiTheme;
    renderStartTime: number;
    /**
     * Define a new theme that can be used to render output.
     * This API will likely be changing at some point, to allow more easily overriding parts of the theme without
     * requiring additional boilerplate.
     * @param name
     * @param theme
     */
    defineTheme(name: string, theme: new (renderer: Renderer) => Theme): void;
    /**
     * Render the given project reflection to the specified output directory.
     *
     * @param project  The project that should be rendered.
     * @param outputDirectory  The path of the directory the documentation should be rendered to.
     */
    render(project: ProjectReflection, outputDirectory: string): Promise<void>;
    /**
     * Render a single page.
     *
     * @param page An event describing the current page.
     * @return TRUE if the page has been saved to disc, otherwise FALSE.
     */
    private renderDocument;
    /**
     * Ensure that a theme has been setup.
     *
     * If a the user has set a theme we try to find and load it. If no theme has
     * been specified we load the default theme.
     *
     * @returns TRUE if a theme has been setup, otherwise FALSE.
     */
    private prepareTheme;
    /**
     * Prepare the output directory. If the directory does not exist, it will be
     * created. If the directory exists, it will be emptied.
     *
     * @param directory  The path to the directory that should be prepared.
     * @returns TRUE if the directory could be prepared, otherwise FALSE.
     */
    private prepareOutputDirectory;
}
import "./plugins";
