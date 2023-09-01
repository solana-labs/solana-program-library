"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.memberSignatureTitle = void 0;
const lib_1 = require("../../lib");
const utils_1 = require("../../../../utils");
const models_1 = require("../../../../models");
function renderParameterWithType(context, item) {
    return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
        !!item.flags.isRest && utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "..."),
        utils_1.JSX.createElement("span", { class: "tsd-kind-parameter" }, item.name),
        utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" },
            !!item.flags.isOptional && "?",
            !!item.defaultValue && "?",
            ": "),
        context.type(item.type)));
}
function renderParameterWithoutType(item) {
    return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
        !!item.flags.isRest && utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "..."),
        utils_1.JSX.createElement("span", { class: "tsd-kind-parameter" }, item.name),
        (item.flags.isOptional || item.defaultValue) && utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "?")));
}
function memberSignatureTitle(context, props, { hideName = false, arrowStyle = false } = {}) {
    const hideParamTypes = context.options.getValue("hideParameterTypesInTitle");
    const renderParam = hideParamTypes ? renderParameterWithoutType : renderParameterWithType.bind(null, context);
    return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
        !hideName ? (utils_1.JSX.createElement("span", { class: (0, lib_1.getKindClass)(props) }, (0, lib_1.wbr)(props.name))) : (utils_1.JSX.createElement(utils_1.JSX.Fragment, null, props.kind === models_1.ReflectionKind.ConstructorSignature && (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            !!props.flags.isAbstract && utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "abstract "),
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "new "))))),
        (0, lib_1.renderTypeParametersSignature)(context, props.typeParameters),
        utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "("),
        (0, lib_1.join)(", ", props.parameters ?? [], renderParam),
        utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ")"),
        !!props.type && (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, arrowStyle ? " => " : ": "),
            context.type(props.type)))));
}
exports.memberSignatureTitle = memberSignatureTitle;
