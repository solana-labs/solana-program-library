"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.DefaultTheme = void 0;
const theme_1 = require("../../theme");
const models_1 = require("../../../models");
const UrlMapping_1 = require("../../models/UrlMapping");
const DefaultThemeRenderContext_1 = require("./DefaultThemeRenderContext");
const utils_1 = require("../../../utils");
const lib_1 = require("../lib");
/**
 * Default theme implementation of TypeDoc. If a theme does not provide a custom
 * {@link Theme} implementation, this theme class will be used.
 */
class DefaultTheme extends theme_1.Theme {
    getRenderContext(pageEvent) {
        return new DefaultThemeRenderContext_1.DefaultThemeRenderContext(this, pageEvent, this.application.options);
    }
    getReflectionClasses(reflection) {
        const filters = this.application.options.getValue("visibilityFilters");
        return getReflectionClasses(reflection, filters);
    }
    /**
     * Create a new DefaultTheme instance.
     *
     * @param renderer  The renderer this theme is attached to.
     * @param basePath  The base path of this theme.
     */
    constructor(renderer) {
        super(renderer);
        this.reflectionTemplate = (pageEvent) => {
            return this.getRenderContext(pageEvent).reflectionTemplate(pageEvent);
        };
        this.indexTemplate = (pageEvent) => {
            return this.getRenderContext(pageEvent).indexTemplate(pageEvent);
        };
        this.defaultLayoutTemplate = (pageEvent, template) => {
            return this.getRenderContext(pageEvent).defaultLayout(template, pageEvent);
        };
        /**
         * Mappings of reflections kinds to templates used by this theme.
         */
        this.mappings = [
            {
                kind: [models_1.ReflectionKind.Class],
                directory: "classes",
                template: this.reflectionTemplate,
            },
            {
                kind: [models_1.ReflectionKind.Interface],
                directory: "interfaces",
                template: this.reflectionTemplate,
            },
            {
                kind: [models_1.ReflectionKind.Enum],
                directory: "enums",
                template: this.reflectionTemplate,
            },
            {
                kind: [models_1.ReflectionKind.Namespace, models_1.ReflectionKind.Module],
                directory: "modules",
                template: this.reflectionTemplate,
            },
            {
                kind: [models_1.ReflectionKind.TypeAlias],
                directory: "types",
                template: this.reflectionTemplate,
            },
            {
                kind: [models_1.ReflectionKind.Function],
                directory: "functions",
                template: this.reflectionTemplate,
            },
            {
                kind: [models_1.ReflectionKind.Variable],
                directory: "variables",
                template: this.reflectionTemplate,
            },
        ];
        this.markedPlugin = renderer.getComponent("marked");
    }
    /**
     * Map the models of the given project to the desired output files.
     *
     * @param project  The project whose urls should be generated.
     * @returns        A list of {@link UrlMapping} instances defining which models
     *                 should be rendered to which files.
     */
    getUrls(project) {
        const urls = [];
        if (false == hasReadme(this.application.options.getValue("readme"))) {
            project.url = "index.html";
            urls.push(new UrlMapping_1.UrlMapping("index.html", project, this.reflectionTemplate));
        }
        else if (project.children?.every((child) => child.kindOf(models_1.ReflectionKind.Module))) {
            // If there are no non-module children, then there's no point in having a modules page since there
            // will be nothing on it besides the navigation, so redirect the module page to the readme page
            project.url = "index.html";
            urls.push(new UrlMapping_1.UrlMapping("index.html", project, this.indexTemplate));
        }
        else {
            project.url = "modules.html";
            urls.push(new UrlMapping_1.UrlMapping("modules.html", project, this.reflectionTemplate));
            urls.push(new UrlMapping_1.UrlMapping("index.html", project, this.indexTemplate));
        }
        project.children?.forEach((child) => {
            if (child instanceof models_1.DeclarationReflection) {
                this.buildUrls(child, urls);
            }
        });
        return urls;
    }
    /**
     * Return a url for the given reflection.
     *
     * @param reflection  The reflection the url should be generated for.
     * @param relative    The parent reflection the url generation should stop on.
     * @param separator   The separator used to generate the url.
     * @returns           The generated url.
     */
    static getUrl(reflection, relative, separator = ".") {
        let url = reflection.getAlias();
        if (reflection.parent && reflection.parent !== relative && !(reflection.parent instanceof models_1.ProjectReflection)) {
            url = DefaultTheme.getUrl(reflection.parent, relative, separator) + separator + url;
        }
        return url;
    }
    /**
     * Return the template mapping for the given reflection.
     *
     * @param reflection  The reflection whose mapping should be resolved.
     * @returns           The found mapping or undefined if no mapping could be found.
     */
    getMapping(reflection) {
        return this.mappings.find((mapping) => reflection.kindOf(mapping.kind));
    }
    /**
     * Build the url for the the given reflection and all of its children.
     *
     * @param reflection  The reflection the url should be created for.
     * @param urls        The array the url should be appended to.
     * @returns           The altered urls array.
     */
    buildUrls(reflection, urls) {
        const mapping = this.getMapping(reflection);
        if (mapping) {
            if (!reflection.url || !DefaultTheme.URL_PREFIX.test(reflection.url)) {
                const url = [mapping.directory, DefaultTheme.getUrl(reflection) + ".html"].join("/");
                urls.push(new UrlMapping_1.UrlMapping(url, reflection, mapping.template));
                reflection.url = url;
                reflection.hasOwnDocument = true;
            }
            reflection.traverse((child) => {
                if (child instanceof models_1.DeclarationReflection) {
                    this.buildUrls(child, urls);
                }
                else {
                    DefaultTheme.applyAnchorUrl(child, reflection);
                }
                return true;
            });
        }
        else if (reflection.parent) {
            DefaultTheme.applyAnchorUrl(reflection, reflection.parent);
        }
        return urls;
    }
    render(page, template) {
        const templateOutput = this.defaultLayoutTemplate(page, template);
        return "<!DOCTYPE html>" + utils_1.JSX.renderElement(templateOutput);
    }
    /**
     * Generate an anchor url for the given reflection and all of its children.
     *
     * @param reflection  The reflection an anchor url should be created for.
     * @param container   The nearest reflection having an own document.
     */
    static applyAnchorUrl(reflection, container) {
        if (!(reflection instanceof models_1.DeclarationReflection) && !(reflection instanceof models_1.SignatureReflection)) {
            return;
        }
        if (!reflection.url || !DefaultTheme.URL_PREFIX.test(reflection.url)) {
            const anchor = DefaultTheme.getUrl(reflection, container, ".");
            reflection.url = container.url + "#" + anchor;
            reflection.anchor = anchor;
            reflection.hasOwnDocument = false;
        }
        reflection.traverse((child) => {
            DefaultTheme.applyAnchorUrl(child, container);
            return true;
        });
    }
}
DefaultTheme.URL_PREFIX = /^(http|ftp)s?:\/\//;
exports.DefaultTheme = DefaultTheme;
function hasReadme(readme) {
    return !readme.endsWith("none");
}
function getReflectionClasses(reflection, filters) {
    const classes = [];
    // Filter classes should match up with the settings function in
    // partials/navigation.tsx.
    for (const key of Object.keys(filters)) {
        if (key === "inherited") {
            if (reflection.inheritedFrom) {
                classes.push("tsd-is-inherited");
            }
        }
        else if (key === "protected") {
            if (reflection.flags.isProtected) {
                classes.push("tsd-is-protected");
            }
        }
        else if (key === "private") {
            if (reflection.flags.isPrivate) {
                classes.push("tsd-is-private");
            }
        }
        else if (key === "external") {
            if (reflection.flags.isExternal) {
                classes.push("tsd-is-external");
            }
        }
        else if (key.startsWith("@")) {
            if (key === "@deprecated") {
                if (reflection.isDeprecated()) {
                    classes.push((0, lib_1.toStyleClass)(`tsd-is-${key.substring(1)}`));
                }
            }
            else if (reflection.comment?.hasModifier(key) ||
                reflection.comment?.getTag(key)) {
                classes.push((0, lib_1.toStyleClass)(`tsd-is-${key.substring(1)}`));
            }
        }
    }
    return classes.join(" ");
}
