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
exports.ContextAwareRendererComponent = exports.RendererComponent = exports.Component = void 0;
const Path = __importStar(require("path"));
const component_1 = require("../utils/component");
Object.defineProperty(exports, "Component", { enumerable: true, get: function () { return component_1.Component; } });
const events_1 = require("./events");
class RendererComponent extends component_1.AbstractComponent {
}
exports.RendererComponent = RendererComponent;
/**
 * A plugin for the renderer that reads the current render context.
 */
class ContextAwareRendererComponent extends RendererComponent {
    constructor() {
        super(...arguments);
        /**
         * Regular expression to test if a string looks like an external url.
         */
        this.urlPrefix = /^(http|ftp)s?:\/\//;
    }
    /**
     * Create a new ContextAwareRendererPlugin instance.
     *
     * @param renderer  The renderer this plugin should be attached to.
     */
    initialize() {
        this.listenTo(this.owner, {
            [events_1.RendererEvent.BEGIN]: this.onBeginRenderer,
            [events_1.PageEvent.BEGIN]: this.onBeginPage,
        });
    }
    /**
     * Transform the given absolute path into a relative path.
     *
     * @param absolute  The absolute path to transform.
     * @returns A path relative to the document currently processed.
     */
    getRelativeUrl(absolute) {
        if (this.urlPrefix.test(absolute)) {
            return absolute;
        }
        else {
            const relative = Path.relative(Path.dirname(this.location), Path.dirname(absolute));
            return Path.join(relative, Path.basename(absolute)).replace(/\\/g, "/");
        }
    }
    /**
     * Triggered before the renderer starts rendering a project.
     *
     * @param event  An event object describing the current render operation.
     */
    onBeginRenderer(event) {
        this.project = event.project;
    }
    /**
     * Triggered before a document will be rendered.
     *
     * @param page  An event object describing the current render operation.
     */
    onBeginPage(page) {
        this.location = page.url;
        this.page = page;
    }
}
exports.ContextAwareRendererComponent = ContextAwareRendererComponent;
