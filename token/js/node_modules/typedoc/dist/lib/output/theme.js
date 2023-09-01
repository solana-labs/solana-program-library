"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Theme = void 0;
const components_1 = require("./components");
const component_1 = require("../utils/component");
/**
 * Base class of all themes.
 *
 * The theme class controls which files will be created through the {@link Theme.getUrls}
 * function. It returns an array of {@link UrlMapping} instances defining the target files, models
 * and templates to use. Additionally themes can subscribe to the events emitted by
 * {@link Renderer} to control and manipulate the output process.
 */
let Theme = class Theme extends components_1.RendererComponent {
    /**
     * Create a new BaseTheme instance.
     *
     * @param renderer  The renderer this theme is attached to.
     */
    constructor(renderer) {
        super(renderer);
    }
};
Theme = __decorate([
    (0, component_1.Component)({ name: "theme", internal: true })
], Theme);
exports.Theme = Theme;
