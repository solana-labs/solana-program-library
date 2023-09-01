"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.breadcrumb = void 0;
const utils_1 = require("../../../../utils");
const breadcrumb = (context, props) => props.parent ? (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
    context.breadcrumb(props.parent),
    utils_1.JSX.createElement("li", null, props.url ? utils_1.JSX.createElement("a", { href: context.urlTo(props) }, props.name) : utils_1.JSX.createElement("span", null, props.name)))) : props.url ? (utils_1.JSX.createElement("li", null,
    utils_1.JSX.createElement("a", { href: context.urlTo(props) }, props.name))) : undefined;
exports.breadcrumb = breadcrumb;
