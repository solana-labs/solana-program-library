"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.anchorIcon = void 0;
const utils_1 = require("../../../../utils");
function anchorIcon(context, anchor) {
    if (!anchor)
        return utils_1.JSX.createElement(utils_1.JSX.Fragment, null);
    return (utils_1.JSX.createElement("a", { href: `#${anchor}`, "aria-label": "Permalink", class: "tsd-anchor-icon" }, context.icons.anchor()));
}
exports.anchorIcon = anchorIcon;
