"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.AssetsPlugin = void 0;
const components_1 = require("../components");
const events_1 = require("../events");
const fs_1 = require("../../utils/fs");
const DefaultTheme_1 = require("../themes/default/DefaultTheme");
const highlighter_1 = require("../../utils/highlighter");
const utils_1 = require("../../utils");
const fs_2 = require("fs");
const path_1 = require("path");
/**
 * A plugin that copies the subdirectory ´assets´ from the current themes
 * source folder to the output directory.
 */
let AssetsPlugin = class AssetsPlugin extends components_1.RendererComponent {
    /**
     * Create a new AssetsPlugin instance.
     */
    initialize() {
        this.listenTo(this.owner, {
            [events_1.RendererEvent.END]: this.onRenderEnd,
            [events_1.RendererEvent.BEGIN]: (event) => {
                const dest = (0, path_1.join)(event.outputDirectory, "assets");
                if (this.customCss) {
                    if ((0, fs_2.existsSync)(this.customCss)) {
                        (0, fs_1.copySync)(this.customCss, (0, path_1.join)(dest, "custom.css"));
                    }
                    else {
                        this.application.logger.error(`Custom CSS file at ${this.customCss} does not exist.`);
                        event.preventDefault();
                    }
                }
            },
        });
    }
    /**
     * Triggered before the renderer starts rendering a project.
     *
     * @param event  An event object describing the current render operation.
     */
    onRenderEnd(event) {
        if (this.owner.theme instanceof DefaultTheme_1.DefaultTheme) {
            const src = (0, path_1.join)(__dirname, "..", "..", "..", "..", "static");
            const dest = (0, path_1.join)(event.outputDirectory, "assets");
            (0, fs_1.copySync)(src, dest);
            (0, fs_1.writeFileSync)((0, path_1.join)(dest, "highlight.css"), (0, highlighter_1.getStyles)());
        }
    }
};
__decorate([
    (0, utils_1.BindOption)("customCss")
], AssetsPlugin.prototype, "customCss", void 0);
AssetsPlugin = __decorate([
    (0, components_1.Component)({ name: "assets" })
], AssetsPlugin);
exports.AssetsPlugin = AssetsPlugin;
