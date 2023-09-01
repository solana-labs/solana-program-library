"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.footer = void 0;
const utils_1 = require("../../../../utils");
function footer(context) {
    const hideGenerator = context.options.getValue("hideGenerator");
    if (!hideGenerator)
        return (utils_1.JSX.createElement("div", { class: "tsd-generator" },
            utils_1.JSX.createElement("p", null,
                "Generated using ",
                utils_1.JSX.createElement("a", { href: "https://typedoc.org/", target: "_blank" }, "TypeDoc"))));
}
exports.footer = footer;
