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
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.MarkedPlugin = void 0;
const fs = __importStar(require("fs"));
const Path = __importStar(require("path"));
const Marked = __importStar(require("marked"));
const components_1 = require("../components");
const events_1 = require("../events");
const utils_1 = require("../../utils");
const highlighter_1 = require("../../utils/highlighter");
const html_1 = require("../../utils/html");
/**
 * Implements markdown and relativeURL helpers for templates.
 * @internal
 */
let MarkedPlugin = class MarkedPlugin extends components_1.ContextAwareRendererComponent {
    constructor() {
        super(...arguments);
        /**
         * The pattern used to find references in markdown.
         */
        this.includePattern = /\[\[include:([^\]]+?)\]\]/g;
        /**
         * The pattern used to find media links.
         */
        this.mediaPattern = /media:\/\/([^ ")\]}]+)/g;
    }
    /**
     * Create a new MarkedPlugin instance.
     */
    initialize() {
        super.initialize();
        this.listenTo(this.owner, events_1.MarkdownEvent.PARSE, this.onParseMarkdown);
    }
    /**
     * Highlight the syntax of the given text using HighlightJS.
     *
     * @param text  The text that should be highlighted.
     * @param lang  The language that should be used to highlight the string.
     * @return A html string with syntax highlighting.
     */
    getHighlighted(text, lang) {
        lang = lang || "typescript";
        lang = lang.toLowerCase();
        if (!(0, highlighter_1.isSupportedLanguage)(lang)) {
            // Extra newline because of the progress bar
            this.application.logger.warn(`
Unsupported highlight language "${lang}" will not be highlighted. Run typedoc --help for a list of supported languages.
target code block :
\t${text.split("\n").join("\n\t")}
source files :${this.sources?.map((source) => `\n\t${source.fileName}`).join()}
output file :
\t${this.outputFileName}`);
            return text;
        }
        return (0, highlighter_1.highlight)(text, lang);
    }
    /**
     * Parse the given markdown string and return the resulting html.
     *
     * @param text  The markdown string that should be parsed.
     * @returns The resulting html string.
     */
    parseMarkdown(text, page) {
        if (this.includes) {
            text = text.replace(this.includePattern, (_match, path) => {
                path = Path.join(this.includes, path.trim());
                if ((0, utils_1.isFile)(path)) {
                    const contents = (0, utils_1.readFile)(path);
                    return contents;
                }
                else {
                    this.application.logger.warn("Could not find file to include: " + path);
                    return "";
                }
            });
        }
        if (this.mediaDirectory) {
            text = text.replace(this.mediaPattern, (match, path) => {
                const fileName = Path.join(this.mediaDirectory, path);
                if ((0, utils_1.isFile)(fileName)) {
                    return this.getRelativeUrl("media") + "/" + path;
                }
                else {
                    this.application.logger.warn("Could not find media file: " + fileName);
                    return match;
                }
            });
        }
        const event = new events_1.MarkdownEvent(events_1.MarkdownEvent.PARSE, page, text, text);
        this.owner.trigger(event);
        return event.parsedText;
    }
    /**
     * Triggered before the renderer starts rendering a project.
     *
     * @param event  An event object describing the current render operation.
     */
    onBeginRenderer(event) {
        super.onBeginRenderer(event);
        Marked.marked.setOptions(this.createMarkedOptions());
        delete this.includes;
        if (this.includeSource) {
            if (fs.existsSync(this.includeSource) &&
                fs.statSync(this.includeSource).isDirectory()) {
                this.includes = this.includeSource;
            }
            else {
                this.application.logger.warn("Could not find provided includes directory: " +
                    this.includeSource);
            }
        }
        if (this.mediaSource) {
            if (fs.existsSync(this.mediaSource) &&
                fs.statSync(this.mediaSource).isDirectory()) {
                this.mediaDirectory = Path.join(event.outputDirectory, "media");
                (0, utils_1.copySync)(this.mediaSource, this.mediaDirectory);
            }
            else {
                this.mediaDirectory = undefined;
                this.application.logger.warn("Could not find provided media directory: " +
                    this.mediaSource);
            }
        }
    }
    /**
     * Creates an object with options that are passed to the markdown parser.
     *
     * @returns The options object for the markdown parser.
     */
    createMarkedOptions() {
        const markedOptions = (this.application.options.getValue("markedOptions") ?? {});
        // Set some default values if they are not specified via the TypeDoc option
        markedOptions.highlight ?? (markedOptions.highlight = (text, lang) => this.getHighlighted(text, lang));
        if (!markedOptions.renderer) {
            markedOptions.renderer = new Marked.Renderer();
            markedOptions.renderer.heading = (text, level, _, slugger) => {
                const slug = slugger.slug(text);
                // Prefix the slug with an extra `md:` to prevent conflicts with TypeDoc's anchors.
                this.page.pageHeadings.push({
                    link: `#md:${slug}`,
                    text: (0, html_1.getTextContent)(text),
                    level,
                });
                return `<a id="md:${slug}" class="tsd-anchor"></a><h${level}><a href="#md:${slug}">${text}</a></h${level}>`;
            };
            markedOptions.renderer.code = renderCode;
        }
        markedOptions.mangle ?? (markedOptions.mangle = false); // See https://github.com/TypeStrong/typedoc/issues/1395
        return markedOptions;
    }
    /**
     * Triggered when {@link MarkedPlugin} parses a markdown string.
     *
     * @param event
     */
    onParseMarkdown(event) {
        event.parsedText = Marked.marked(event.parsedText);
    }
};
__decorate([
    (0, utils_1.BindOption)("includes")
], MarkedPlugin.prototype, "includeSource", void 0);
__decorate([
    (0, utils_1.BindOption)("media")
], MarkedPlugin.prototype, "mediaSource", void 0);
__decorate([
    (0, utils_1.BindOption)("lightHighlightTheme")
], MarkedPlugin.prototype, "lightTheme", void 0);
__decorate([
    (0, utils_1.BindOption)("darkHighlightTheme")
], MarkedPlugin.prototype, "darkTheme", void 0);
MarkedPlugin = __decorate([
    (0, components_1.Component)({ name: "marked" })
], MarkedPlugin);
exports.MarkedPlugin = MarkedPlugin;
// Basically a copy/paste of Marked's code, with the addition of the button
// https://github.com/markedjs/marked/blob/v4.3.0/src/Renderer.js#L15-L39
function renderCode(code, infostring, escaped) {
    const lang = (infostring || "").match(/\S*/)[0];
    if (this.options.highlight) {
        const out = this.options.highlight(code, lang);
        if (out != null && out !== code) {
            escaped = true;
            code = out;
        }
    }
    code = code.replace(/\n$/, "") + "\n";
    if (!lang) {
        return `<pre><code>${escaped ? code : (0, html_1.escapeHtml)(code)}</code><button>Copy</button></pre>\n`;
    }
    return `<pre><code class="${this.options.langPrefix + (0, html_1.escapeHtml)(lang)}">${escaped ? code : (0, html_1.escapeHtml)(code)}</code><button>Copy</button></pre>\n`;
}
