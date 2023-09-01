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
exports.JavascriptIndexPlugin = void 0;
const Path = __importStar(require("path"));
const lunr_1 = require("lunr");
const models_1 = require("../../models");
const components_1 = require("../components");
const events_1 = require("../events");
const utils_1 = require("../../utils");
const DefaultTheme_1 = require("../themes/default/DefaultTheme");
/**
 * A plugin that exports an index of the project to a javascript file.
 *
 * The resulting javascript file can be used to build a simple search function.
 */
let JavascriptIndexPlugin = class JavascriptIndexPlugin extends components_1.RendererComponent {
    /**
     * Create a new JavascriptIndexPlugin instance.
     */
    initialize() {
        this.listenTo(this.owner, events_1.RendererEvent.BEGIN, this.onRendererBegin);
    }
    /**
     * Triggered after a document has been rendered, just before it is written to disc.
     *
     * @param event  An event object describing the current render operation.
     */
    onRendererBegin(event) {
        if (!(this.owner.theme instanceof DefaultTheme_1.DefaultTheme)) {
            return;
        }
        if (event.isDefaultPrevented) {
            return;
        }
        const rows = [];
        const initialSearchResults = Object.values(event.project.reflections).filter((refl) => {
            return (refl instanceof models_1.DeclarationReflection &&
                refl.url &&
                refl.name &&
                !refl.flags.isExternal);
        });
        const indexEvent = new events_1.IndexEvent(events_1.IndexEvent.PREPARE_INDEX, initialSearchResults);
        this.owner.trigger(indexEvent);
        if (indexEvent.isDefaultPrevented) {
            return;
        }
        const builder = new lunr_1.Builder();
        builder.pipeline.add(lunr_1.trimmer);
        builder.ref("id");
        for (const [key, boost] of Object.entries(indexEvent.searchFieldWeights)) {
            builder.field(key, { boost });
        }
        for (const reflection of indexEvent.searchResults) {
            if (!reflection.url) {
                continue;
            }
            const boost = reflection.relevanceBoost ?? 1;
            if (boost <= 0) {
                continue;
            }
            let parent = reflection.parent;
            if (parent instanceof models_1.ProjectReflection) {
                parent = undefined;
            }
            const row = {
                kind: reflection.kind,
                name: reflection.name,
                url: reflection.url,
                classes: this.owner.theme.getReflectionClasses(reflection),
            };
            if (parent) {
                row.parent = parent.getFullName();
            }
            builder.add({
                name: reflection.name,
                comment: this.getCommentSearchText(reflection),
                ...indexEvent.searchFields[rows.length],
                id: rows.length,
            }, { boost });
            rows.push(row);
        }
        const index = builder.build();
        const jsonFileName = Path.join(event.outputDirectory, "assets", "search.js");
        const jsonData = JSON.stringify({
            rows,
            index,
        });
        (0, utils_1.writeFileSync)(jsonFileName, `window.searchData = JSON.parse(${JSON.stringify(jsonData)});`);
    }
    getCommentSearchText(reflection) {
        if (!this.searchComments)
            return;
        const comments = [];
        if (reflection.comment)
            comments.push(reflection.comment);
        reflection.signatures?.forEach((s) => s.comment && comments.push(s.comment));
        reflection.getSignature?.comment &&
            comments.push(reflection.getSignature.comment);
        reflection.setSignature?.comment &&
            comments.push(reflection.setSignature.comment);
        if (!comments.length) {
            return;
        }
        return comments
            .flatMap((c) => {
            return [...c.summary, ...c.blockTags.flatMap((t) => t.content)];
        })
            .map((part) => part.text)
            .join("\n");
    }
};
__decorate([
    (0, utils_1.BindOption)("searchInComments")
], JavascriptIndexPlugin.prototype, "searchComments", void 0);
JavascriptIndexPlugin = __decorate([
    (0, components_1.Component)({ name: "javascript-index" })
], JavascriptIndexPlugin);
exports.JavascriptIndexPlugin = JavascriptIndexPlugin;
