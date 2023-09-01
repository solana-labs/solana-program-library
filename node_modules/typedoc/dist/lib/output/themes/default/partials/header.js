"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.header = void 0;
const lib_1 = require("../../lib");
const utils_1 = require("../../../../utils");
const models_1 = require("../../../../models");
const header = (context, props) => {
    const HeadingLevel = props.model.isProject() ? "h2" : "h1";
    return (utils_1.JSX.createElement("div", { class: "tsd-page-title" },
        !!props.model.parent && utils_1.JSX.createElement("ul", { class: "tsd-breadcrumb" }, context.breadcrumb(props.model)),
        utils_1.JSX.createElement(HeadingLevel, null,
            props.model.kind !== models_1.ReflectionKind.Project && `${models_1.ReflectionKind.singularString(props.model.kind)} `,
            (0, lib_1.getDisplayName)(props.model),
            (0, lib_1.hasTypeParameters)(props.model) && (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                "<",
                (0, lib_1.join)(", ", props.model.typeParameters, (item) => item.name),
                ">")),
            (0, lib_1.renderFlags)(props.model.flags, props.model.comment))));
};
exports.header = header;
