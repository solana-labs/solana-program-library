"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.index = void 0;
const lib_1 = require("../../lib");
const utils_1 = require("../../../../utils");
function renderCategory({ urlTo, icons, getReflectionClasses }, item, prependName = "") {
    return (utils_1.JSX.createElement("section", { class: "tsd-index-section" },
        utils_1.JSX.createElement("h3", { class: "tsd-index-heading" }, prependName ? `${prependName} - ${item.title}` : item.title),
        utils_1.JSX.createElement("div", { class: "tsd-index-list" }, item.children.map((item) => (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("a", { href: urlTo(item), class: (0, lib_1.classNames)({ "tsd-index-link": true, deprecated: item.isDeprecated() }, getReflectionClasses(item)) },
                icons[item.kind](),
                utils_1.JSX.createElement("span", null, (0, lib_1.renderName)(item))),
            "\n"))))));
}
function index(context, props) {
    let content = [];
    if (props.categories?.length) {
        content = props.categories.map((item) => renderCategory(context, item));
    }
    else if (props.groups?.length) {
        content = props.groups.flatMap((item) => item.categories
            ? item.categories.map((item2) => renderCategory(context, item2, item.title))
            : renderCategory(context, item));
    }
    // Accordion is only needed if any children don't have their own document.
    if ([...(props.groups ?? []), ...(props.categories ?? [])].some((category) => !category.allChildrenHaveOwnDocument())) {
        content = (utils_1.JSX.createElement("details", { class: "tsd-index-content tsd-index-accordion", open: true },
            utils_1.JSX.createElement("summary", { class: "tsd-accordion-summary tsd-index-summary" },
                utils_1.JSX.createElement("h5", { class: "tsd-index-heading uppercase", role: "button", "aria-expanded": "false", tabIndex: 0 },
                    context.icons.chevronSmall(),
                    " Index")),
            utils_1.JSX.createElement("div", { class: "tsd-accordion-details" }, content)));
    }
    else {
        content = (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("h3", { class: "tsd-index-heading uppercase" }, "Index"),
            content));
    }
    return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
        utils_1.JSX.createElement("section", { class: "tsd-panel-group tsd-index-group" },
            utils_1.JSX.createElement("section", { class: "tsd-panel tsd-index-panel" }, content))));
}
exports.index = index;
