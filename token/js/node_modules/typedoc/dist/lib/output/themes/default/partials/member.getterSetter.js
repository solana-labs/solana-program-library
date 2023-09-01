"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.memberGetterSetter = void 0;
const utils_1 = require("../../../../utils");
const lib_1 = require("../../lib");
const memberGetterSetter = (context, props) => (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
    utils_1.JSX.createElement("ul", { class: (0, lib_1.classNames)({
            "tsd-signatures": true,
        }, context.getReflectionClasses(props)) },
        !!props.getSignature && (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("li", { class: "tsd-signature", id: props.getSignature.anchor },
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "get"),
                " ",
                props.name,
                context.memberSignatureTitle(props.getSignature, {
                    hideName: true,
                })),
            utils_1.JSX.createElement("li", { class: "tsd-description" }, context.memberSignatureBody(props.getSignature)))),
        !!props.setSignature && (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("li", { class: "tsd-signature", id: props.setSignature.anchor },
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "set"),
                " ",
                props.name,
                context.memberSignatureTitle(props.setSignature, {
                    hideName: true,
                })),
            utils_1.JSX.createElement("li", { class: "tsd-description" }, context.memberSignatureBody(props.setSignature)))))));
exports.memberGetterSetter = memberGetterSetter;
