"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.memberSignatures = void 0;
const utils_1 = require("../../../../utils");
const anchor_icon_1 = require("./anchor-icon");
const lib_1 = require("../../lib");
const memberSignatures = (context, props) => (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
    utils_1.JSX.createElement("ul", { class: (0, lib_1.classNames)({ "tsd-signatures": true }, context.getReflectionClasses(props)) }, props.signatures?.map((item) => (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
        utils_1.JSX.createElement("li", { class: "tsd-signature tsd-anchor-link", id: item.anchor },
            context.memberSignatureTitle(item),
            (0, anchor_icon_1.anchorIcon)(context, item.anchor)),
        utils_1.JSX.createElement("li", { class: "tsd-description" }, context.memberSignatureBody(item))))))));
exports.memberSignatures = memberSignatures;
