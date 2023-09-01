"use strict";
/**
 * Custom JSX module designed specifically for TypeDoc's needs.
 * When overriding a default TypeDoc theme output, your implementation must create valid {@link Element}
 * instances, which can be most easily done by using TypeDoc's JSX implementation. To use it, set up
 * your tsconfig with the following compiler options:
 * ```json
 * {
 *     "jsx": "react",
 *     "jsxFactory": "JSX.createElement",
 *     "jsxFragmentFactory": "JSX.Fragment"
 * }
 * ```
 * @module
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.renderElement = exports.createElement = exports.Raw = exports.Fragment = void 0;
const html_1 = require("./html");
const jsx_elements_1 = require("./jsx.elements");
var jsx_elements_2 = require("./jsx.elements");
Object.defineProperty(exports, "Fragment", { enumerable: true, get: function () { return jsx_elements_2.JsxFragment; } });
/**
 * Used to inject HTML directly into the document.
 */
function Raw(_props) {
    // This is handled specially by the renderElement function. Instead of being
    // called, the tag is compared to this function and the `html` prop will be
    // returned directly.
    return null;
}
exports.Raw = Raw;
const voidElements = new Set([
    "area",
    "base",
    "br",
    "col",
    "embed",
    "hr",
    "img",
    "input",
    "link",
    "meta",
    "param",
    "source",
    "track",
    "wbr",
]);
const blockElements = new Set([
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "div",
    "section",
    "nav",
    "details",
    "p",
    "ul",
    "ol",
    "li",
]);
/**
 * JSX factory function to create an "element" that can later be rendered with {@link renderElement}
 * @param tag
 * @param props
 * @param children
 */
function createElement(tag, props, ...children) {
    return { tag, props, children };
}
exports.createElement = createElement;
function renderElement(element) {
    if (!element) {
        return "";
    }
    const { tag, props, children } = element;
    if (typeof tag === "function") {
        if (tag === Raw) {
            return String(props.html);
        }
        return renderElement(tag(Object.assign({ children }, props)));
    }
    const html = [];
    if (tag !== jsx_elements_1.JsxFragment) {
        if (blockElements.has(tag)) {
            html.push("\n");
        }
        html.push("<", tag);
        for (const [key, val] of Object.entries(props ?? {})) {
            if (val == null)
                continue;
            if (typeof val == "boolean") {
                if (val) {
                    html.push(" ", key);
                }
            }
            else {
                html.push(" ", key, "=", JSON.stringify(val));
            }
        }
    }
    let hasChildren = false;
    if (children.length) {
        hasChildren = true;
        if (tag !== jsx_elements_1.JsxFragment)
            html.push(">");
        renderChildren(children);
    }
    if (tag !== jsx_elements_1.JsxFragment) {
        if (!hasChildren) {
            if (voidElements.has(tag)) {
                html.push("/>");
            }
            else {
                html.push("></", tag, ">");
            }
        }
        else {
            html.push("</", tag, ">");
        }
    }
    return html.join("");
    function renderChildren(children) {
        for (const child of children) {
            if (!child)
                continue;
            if (Array.isArray(child)) {
                renderChildren(child);
            }
            else if (typeof child === "string" || typeof child === "number") {
                html.push((0, html_1.escapeHtml)(child.toString()));
            }
            else {
                html.push(renderElement(child));
            }
        }
    }
}
exports.renderElement = renderElement;
