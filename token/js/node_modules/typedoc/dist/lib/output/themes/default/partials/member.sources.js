"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.memberSources = void 0;
const utils_1 = require("../../../../utils");
const memberSources = (context, props) => {
    const sources = [];
    if (props.implementationOf) {
        sources.push(utils_1.JSX.createElement("p", null,
            "Implementation of ",
            context.typeAndParent(props.implementationOf)));
    }
    if (props.inheritedFrom) {
        sources.push(utils_1.JSX.createElement("p", null,
            "Inherited from ",
            context.typeAndParent(props.inheritedFrom)));
    }
    if (props.overwrites) {
        sources.push(utils_1.JSX.createElement("p", null,
            "Overrides ",
            context.typeAndParent(props.overwrites)));
    }
    if (props.sources) {
        sources.push(utils_1.JSX.createElement("ul", null, props.sources.map((item) => item.url ? (utils_1.JSX.createElement("li", null,
            "Defined in ",
            utils_1.JSX.createElement("a", { href: item.url },
                item.fileName,
                ":",
                item.line))) : (utils_1.JSX.createElement("li", null,
            "Defined in ",
            item.fileName,
            ":",
            item.line)))));
    }
    if (sources.length === 0) {
        return utils_1.JSX.createElement(utils_1.JSX.Fragment, null);
    }
    return utils_1.JSX.createElement("aside", { class: "tsd-sources" }, sources);
};
exports.memberSources = memberSources;
