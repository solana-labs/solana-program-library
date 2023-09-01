"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.comment = void 0;
const utils_1 = require("../../../../utils");
const models_1 = require("../../../../models");
const lib_1 = require("../../lib");
function comment({ markdown }, props) {
    if (!props.comment?.hasVisibleComponent())
        return;
    // Note: Comment modifiers are handled in `renderFlags`
    const tags = props.kindOf(models_1.ReflectionKind.SomeSignature)
        ? props.comment.blockTags.filter((tag) => tag.tag !== "@returns")
        : props.comment.blockTags;
    return (utils_1.JSX.createElement("div", { class: "tsd-comment tsd-typography" },
        utils_1.JSX.createElement(utils_1.Raw, { html: markdown(props.comment.summary) }),
        tags.map((item) => (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("h3", null, (0, lib_1.camelToTitleCase)(item.tag.substring(1))),
            utils_1.JSX.createElement(utils_1.Raw, { html: markdown(item.content) }))))));
}
exports.comment = comment;
