"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.toolbar = void 0;
const utils_1 = require("../../../../utils");
const lib_1 = require("../../lib");
const toolbar = (context, props) => (utils_1.JSX.createElement("header", { class: "tsd-page-toolbar" },
    utils_1.JSX.createElement("div", { class: "tsd-toolbar-contents container" },
        utils_1.JSX.createElement("div", { class: "table-cell", id: "tsd-search", "data-base": context.relativeURL("./") },
            utils_1.JSX.createElement("div", { class: "field" },
                utils_1.JSX.createElement("label", { for: "tsd-search-field", class: "tsd-widget tsd-toolbar-icon search no-caption" }, context.icons.search()),
                utils_1.JSX.createElement("input", { type: "text", id: "tsd-search-field", "aria-label": "Search" })),
            utils_1.JSX.createElement("div", { class: "field" },
                utils_1.JSX.createElement("div", { id: "tsd-toolbar-links" }, Object.entries(context.options.getValue("navigationLinks")).map(([label, url]) => (utils_1.JSX.createElement("a", { href: url }, label))))),
            utils_1.JSX.createElement("ul", { class: "results" },
                utils_1.JSX.createElement("li", { class: "state loading" }, "Preparing search index..."),
                utils_1.JSX.createElement("li", { class: "state failure" }, "The search index is not available")),
            utils_1.JSX.createElement("a", { href: context.options.getValue("titleLink") || context.relativeURL("index.html"), class: "title" }, (0, lib_1.getDisplayName)(props.project))),
        utils_1.JSX.createElement("div", { class: "table-cell", id: "tsd-widgets" },
            utils_1.JSX.createElement("a", { href: "#", class: "tsd-widget tsd-toolbar-icon menu no-caption", "data-toggle": "menu", "aria-label": "Menu" }, context.icons.menu())))));
exports.toolbar = toolbar;
