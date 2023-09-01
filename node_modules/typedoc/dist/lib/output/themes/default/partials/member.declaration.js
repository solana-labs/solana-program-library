"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.memberDeclaration = void 0;
const models_1 = require("../../../../models");
const utils_1 = require("../../../../utils");
const lib_1 = require("../../lib");
const memberDeclaration = (context, props) => (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
    utils_1.JSX.createElement("div", { class: "tsd-signature" },
        utils_1.JSX.createElement("span", { class: (0, lib_1.getKindClass)(props) }, (0, lib_1.wbr)(props.name)),
        (0, lib_1.renderTypeParametersSignature)(context, props.typeParameters),
        props.type && (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" },
                !!props.flags.isOptional && "?",
                ":"),
            " ",
            context.type(props.type))),
        !!props.defaultValue && (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" },
                " = ",
                props.defaultValue)))),
    context.comment(props),
    (0, lib_1.hasTypeParameters)(props) && context.typeParameters(props.typeParameters),
    props.type instanceof models_1.ReflectionType && (utils_1.JSX.createElement("div", { class: "tsd-type-declaration" },
        utils_1.JSX.createElement("h4", null, "Type declaration"),
        context.parameter(props.type.declaration))),
    context.memberSources(props)));
exports.memberDeclaration = memberDeclaration;
