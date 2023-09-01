"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.pageNavigation = exports.pageSidebar = exports.navigation = exports.settings = exports.sidebarLinks = exports.sidebar = void 0;
const models_1 = require("../../../../models");
const utils_1 = require("../../../../utils");
const lib_1 = require("../../lib");
function sidebar(context, props) {
    return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
        context.sidebarLinks(),
        context.navigation(props)));
}
exports.sidebar = sidebar;
function buildFilterItem(context, name, displayName, defaultValue) {
    return (utils_1.JSX.createElement("li", { class: "tsd-filter-item" },
        utils_1.JSX.createElement("label", { class: "tsd-filter-input" },
            utils_1.JSX.createElement("input", { type: "checkbox", id: `tsd-filter-${name}`, name: name, checked: defaultValue }),
            context.icons.checkbox(),
            utils_1.JSX.createElement("span", null, displayName))));
}
function sidebarLinks(context) {
    const links = Object.entries(context.options.getValue("sidebarLinks"));
    if (!links.length)
        return null;
    return (utils_1.JSX.createElement("nav", { id: "tsd-sidebar-links", class: "tsd-navigation" }, links.map(([label, url]) => (utils_1.JSX.createElement("a", { href: url, target: "_blank" }, label)))));
}
exports.sidebarLinks = sidebarLinks;
function settings(context) {
    const defaultFilters = context.options.getValue("visibilityFilters");
    const visibilityOptions = [];
    for (const key of Object.keys(defaultFilters)) {
        if (key.startsWith("@")) {
            const filterName = key
                .substring(1)
                .replace(/([a-z])([A-Z])/g, "$1-$2")
                .toLowerCase();
            visibilityOptions.push(buildFilterItem(context, filterName, (0, lib_1.camelToTitleCase)(key.substring(1)), defaultFilters[key]));
        }
        else if ((key === "protected" && !context.options.getValue("excludeProtected")) ||
            (key === "private" && !context.options.getValue("excludePrivate")) ||
            (key === "external" && !context.options.getValue("excludeExternals")) ||
            key === "inherited") {
            visibilityOptions.push(buildFilterItem(context, key, (0, lib_1.camelToTitleCase)(key), defaultFilters[key]));
        }
    }
    // Settings panel above navigation
    return (utils_1.JSX.createElement("div", { class: "tsd-navigation settings" },
        utils_1.JSX.createElement("details", { class: "tsd-index-accordion", open: false },
            utils_1.JSX.createElement("summary", { class: "tsd-accordion-summary" },
                utils_1.JSX.createElement("h3", null,
                    context.icons.chevronDown(),
                    "Settings")),
            utils_1.JSX.createElement("div", { class: "tsd-accordion-details" },
                visibilityOptions.length && (utils_1.JSX.createElement("div", { class: "tsd-filter-visibility" },
                    utils_1.JSX.createElement("h4", { class: "uppercase" }, "Member Visibility"),
                    utils_1.JSX.createElement("form", null,
                        utils_1.JSX.createElement("ul", { id: "tsd-filter-options" }, ...visibilityOptions)))),
                utils_1.JSX.createElement("div", { class: "tsd-theme-toggle" },
                    utils_1.JSX.createElement("h4", { class: "uppercase" }, "Theme"),
                    utils_1.JSX.createElement("select", { id: "tsd-theme" },
                        utils_1.JSX.createElement("option", { value: "os" }, "OS"),
                        utils_1.JSX.createElement("option", { value: "light" }, "Light"),
                        utils_1.JSX.createElement("option", { value: "dark" }, "Dark")))))));
}
exports.settings = settings;
function getNavigationElements(parent, opts) {
    if (parent instanceof models_1.ReflectionCategory) {
        return parent.children;
    }
    if (parent instanceof models_1.ReflectionGroup) {
        if (opts.includeCategories && parent.categories) {
            return parent.categories;
        }
        return parent.children;
    }
    if (!parent.kindOf(models_1.ReflectionKind.SomeModule | models_1.ReflectionKind.Project)) {
        return [];
    }
    if (parent.categories && opts.includeCategories) {
        return parent.categories;
    }
    if (parent.groups && opts.includeGroups) {
        return parent.groups;
    }
    return parent.children || [];
}
function navigation(context, props) {
    const opts = context.options.getValue("navigation");
    // Create the navigation for the current page
    // Recurse to children if the parent is some kind of module
    return (utils_1.JSX.createElement("nav", { class: "tsd-navigation" },
        createNavElement(props.project),
        utils_1.JSX.createElement("ul", { class: "tsd-small-nested-navigation" }, getNavigationElements(props.project, opts).map((c) => (utils_1.JSX.createElement("li", null, links(c, [])))))));
    function links(mod, parents) {
        const nameClasses = (0, lib_1.classNames)({ deprecated: mod instanceof models_1.Reflection && mod.isDeprecated() }, !(mod instanceof models_1.Reflection) || mod.isProject() ? void 0 : context.getReflectionClasses(mod));
        const children = getNavigationElements(mod, opts);
        if (!children.length) {
            return createNavElement(mod, nameClasses);
        }
        return (utils_1.JSX.createElement("details", { class: (0, lib_1.classNames)({ "tsd-index-accordion": true }, nameClasses), open: mod instanceof models_1.Reflection && inPath(mod), "data-key": mod instanceof models_1.Reflection ? mod.getFullName() : [...parents, mod.title].join("$") },
            utils_1.JSX.createElement("summary", { class: "tsd-accordion-summary" },
                context.icons.chevronDown(),
                createNavElement(mod)),
            utils_1.JSX.createElement("div", { class: "tsd-accordion-details" },
                utils_1.JSX.createElement("ul", { class: "tsd-nested-navigation" }, children.map((c) => (utils_1.JSX.createElement("li", null, links(c, mod instanceof models_1.Reflection ? [mod.getFullName()] : [...parents, mod.title]))))))));
    }
    function createNavElement(child, nameClasses) {
        if (child instanceof models_1.Reflection) {
            return (utils_1.JSX.createElement("a", { href: context.urlTo(child), class: (0, lib_1.classNames)({ current: child === props.model }, nameClasses) },
                context.icons[child.kind](),
                utils_1.JSX.createElement("span", null, (0, lib_1.wbr)((0, lib_1.getDisplayName)(child)))));
        }
        return utils_1.JSX.createElement("span", null, child.title);
    }
    function inPath(mod) {
        let iter = props.model;
        do {
            if (iter == mod)
                return true;
            iter = iter.parent;
        } while (iter);
        return false;
    }
}
exports.navigation = navigation;
function pageSidebar(context, props) {
    return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
        context.settings(),
        context.pageNavigation(props)));
}
exports.pageSidebar = pageSidebar;
function pageNavigation(context, props) {
    const levels = [[]];
    function finalizeLevel() {
        const built = (utils_1.JSX.createElement("ul", null, levels.pop().map((l) => (utils_1.JSX.createElement("li", null, l)))));
        levels[levels.length - 1].push(built);
    }
    for (const heading of props.pageHeadings) {
        const inferredLevel = heading.level ? heading.level + 1 : 1;
        while (inferredLevel < levels.length) {
            finalizeLevel();
        }
        if (inferredLevel > levels.length) {
            // Lower level than before
            levels.push([]);
        }
        levels[levels.length - 1].push(utils_1.JSX.createElement("a", { href: heading.link, class: heading.classes },
            heading.kind && context.icons[heading.kind](),
            utils_1.JSX.createElement("span", null, (0, lib_1.wbr)(heading.text))));
    }
    while (levels.length > 1) {
        finalizeLevel();
    }
    if (!levels[0].length) {
        return utils_1.JSX.createElement(utils_1.JSX.Fragment, null);
    }
    return (utils_1.JSX.createElement("details", { open: true, class: "tsd-index-accordion tsd-page-navigation" },
        utils_1.JSX.createElement("summary", { class: "tsd-accordion-summary" },
            utils_1.JSX.createElement("h3", null,
                context.icons.chevronDown(),
                "On This Page")),
        utils_1.JSX.createElement("div", { class: "tsd-accordion-details" },
            utils_1.JSX.createElement("ul", null, levels[0].map((l) => (utils_1.JSX.createElement("li", null, l)))))));
}
exports.pageNavigation = pageNavigation;
