"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.hierarchy = void 0;
const utils_1 = require("../../../../utils");
function hierarchy(context, props) {
    if (!props)
        return;
    return (utils_1.JSX.createElement("section", { class: "tsd-panel tsd-hierarchy" },
        utils_1.JSX.createElement("h4", null, "Hierarchy"),
        hierarchyList(context, props)));
}
exports.hierarchy = hierarchy;
function hierarchyList(context, props) {
    return (utils_1.JSX.createElement("ul", { class: "tsd-hierarchy" }, props.types.map((item, i, l) => (utils_1.JSX.createElement("li", null,
        props.isTarget ? utils_1.JSX.createElement("span", { class: "target" }, item.toString()) : context.type(item),
        i === l.length - 1 && !!props.next && hierarchyList(context, props.next))))));
}
