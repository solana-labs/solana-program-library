"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.membersGroup = void 0;
const utils_1 = require("../../../../utils");
function membersGroup(context, group) {
    if (group.categories) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null, group.categories.map((item) => (utils_1.JSX.createElement("section", { class: "tsd-panel-group tsd-member-group" },
            utils_1.JSX.createElement("h2", null,
                group.title,
                !!item.title && utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                    " - ",
                    item.title)),
            item.children.map((item) => !item.hasOwnDocument && context.member(item)))))));
    }
    return (utils_1.JSX.createElement("section", { class: "tsd-panel-group tsd-member-group" },
        utils_1.JSX.createElement("h2", null, group.title),
        group.children.map((item) => !item.hasOwnDocument && context.member(item))));
}
exports.membersGroup = membersGroup;
