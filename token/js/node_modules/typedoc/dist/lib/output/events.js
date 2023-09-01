"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.IndexEvent = exports.MarkdownEvent = exports.PageEvent = exports.RendererEvent = void 0;
const Path = __importStar(require("path"));
const events_1 = require("../utils/events");
/**
 * An event emitted by the {@link Renderer} class at the very beginning and
 * ending of the entire rendering process.
 *
 * @see {@link Renderer.EVENT_BEGIN}
 * @see {@link Renderer.EVENT_END}
 */
class RendererEvent extends events_1.Event {
    constructor(name, outputDirectory, project) {
        super(name);
        this.outputDirectory = outputDirectory;
        this.project = project;
    }
    /**
     * Create an {@link PageEvent} event based on this event and the given url mapping.
     *
     * @internal
     * @param mapping  The mapping that defines the generated {@link PageEvent} state.
     * @returns A newly created {@link PageEvent} instance.
     */
    createPageEvent(mapping) {
        const event = new PageEvent(PageEvent.BEGIN, mapping.model);
        event.project = this.project;
        event.url = mapping.url;
        event.filename = Path.join(this.outputDirectory, mapping.url);
        return [mapping.template, event];
    }
}
/**
 * Triggered before the renderer starts rendering a project.
 * @event
 */
RendererEvent.BEGIN = "beginRender";
/**
 * Triggered after the renderer has written all documents.
 * @event
 */
RendererEvent.END = "endRender";
exports.RendererEvent = RendererEvent;
/**
 * An event emitted by the {@link Renderer} class before and after the
 * markup of a page is rendered.
 *
 * @see {@link Renderer.EVENT_BEGIN_PAGE}
 * @see {@link Renderer.EVENT_END_PAGE}
 */
class PageEvent extends events_1.Event {
    constructor(name, model) {
        super(name);
        /**
         * Links to content within this page that should be rendered in the page navigation.
         * This is built when rendering the document content.
         */
        this.pageHeadings = [];
        this.model = model;
    }
}
/**
 * Triggered before a document will be rendered.
 * @event
 */
PageEvent.BEGIN = "beginPage";
/**
 * Triggered after a document has been rendered, just before it is written to disc.
 * @event
 */
PageEvent.END = "endPage";
exports.PageEvent = PageEvent;
/**
 * An event emitted when markdown is being parsed. Allows other plugins to manipulate the result.
 *
 * @see {@link MarkdownEvent.PARSE}
 */
class MarkdownEvent extends events_1.Event {
    constructor(name, page, originalText, parsedText) {
        super(name);
        this.page = page;
        this.originalText = originalText;
        this.parsedText = parsedText;
    }
}
/**
 * Triggered on the renderer when this plugin parses a markdown string.
 * @event
 */
MarkdownEvent.PARSE = "parseMarkdown";
exports.MarkdownEvent = MarkdownEvent;
/**
 * An event emitted when the search index is being prepared.
 */
class IndexEvent extends events_1.Event {
    /**
     * Remove a search result by index.
     */
    removeResult(index) {
        this.searchResults.splice(index, 1);
        this.searchFields.splice(index, 1);
    }
    constructor(name, searchResults) {
        super(name);
        /**
         * Weights for the fields defined in `searchFields`. The default will weight
         * `name` as 10x more important than comment content.
         *
         * If a field added to {@link searchFields} is not added to this object, it
         * will **not** be searchable.
         *
         * Do not replace this object, instead, set new properties on it for custom search
         * fields added by your plugin.
         */
        this.searchFieldWeights = {
            name: 10,
            comment: 1,
        };
        this.searchResults = searchResults;
        this.searchFields = Array.from({ length: this.searchResults.length }, () => ({}));
    }
}
/**
 * Triggered on the renderer when the search index is being prepared.
 * @event
 */
IndexEvent.PREPARE_INDEX = "prepareIndex";
exports.IndexEvent = IndexEvent;
