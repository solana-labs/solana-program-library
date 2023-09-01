"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.escapeHtml = exports.getTextContent = void 0;
// There is a fixed list of named character references which will not be expanded in the future.
// This json file is based on https://html.spec.whatwg.org/multipage/named-characters.html#named-character-references
// with some modifications to reduce the file size of the original JSON since we just need.
const html_entities_json_1 = __importDefault(require("./html-entities.json"));
// Three cases:
// &#123; - numeric escape
// &#x12; - hex escape
// &amp; - named escape
function unescapeEntities(html) {
    return html.replace(/&(#(?:\d+);?|(?:#[xX][0-9A-Fa-f]+);?|(?:\w+);?)/g, (_, n) => {
        if (n[0] === "#") {
            return String.fromCharCode(n[1] === "x" || n[1] === "X"
                ? parseInt(n.substring(2), 16)
                : parseInt(n.substring(1), 10));
        }
        return html_entities_json_1.default[n] || "";
    });
}
function getTextContent(text) {
    return unescapeEntities(text.replace(/<.*?(?:>|$)/g, ""));
}
exports.getTextContent = getTextContent;
const htmlEscapes = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
};
function escapeHtml(html) {
    return html.replace(/[&<>'"]/g, (c) => htmlEscapes[c]);
}
exports.escapeHtml = escapeHtml;
