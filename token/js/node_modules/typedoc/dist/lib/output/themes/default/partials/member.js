"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.member = void 0;
const lib_1 = require("../../lib");
const utils_1 = require("../../../../utils");
const models_1 = require("../../../../models");
const anchor_icon_1 = require("./anchor-icon");
function member(context, props) {
    context.page.pageHeadings.push({
        link: `#${props.anchor}`,
        text: (0, lib_1.getDisplayName)(props),
        kind: props.kind,
        classes: context.getReflectionClasses(props),
    });
    return (utils_1.JSX.createElement("section", { class: (0, lib_1.classNames)({ "tsd-panel": true, "tsd-member": true }, context.getReflectionClasses(props)) },
        utils_1.JSX.createElement("a", { id: props.anchor, class: "tsd-anchor" }),
        !!props.name && (utils_1.JSX.createElement("h3", { class: "tsd-anchor-link" },
            (0, lib_1.renderFlags)(props.flags, props.comment),
            utils_1.JSX.createElement("span", { class: (0, lib_1.classNames)({ deprecated: props.isDeprecated() }) }, (0, lib_1.wbr)(props.name)),
            (0, anchor_icon_1.anchorIcon)(context, props.anchor))),
        props.signatures
            ? context.memberSignatures(props)
            : props.hasGetterOrSetter()
                ? context.memberGetterSetter(props)
                : props instanceof models_1.ReferenceReflection
                    ? context.memberReference(props)
                    : context.memberDeclaration(props),
        props.groups?.map((item) => item.children.map((item) => !item.hasOwnDocument && context.member(item)))));
}
exports.member = member;
