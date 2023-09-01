"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.getStyles = exports.highlight = exports.getSupportedLanguages = exports.isSupportedLanguage = exports.loadHighlighter = void 0;
const assert_1 = require("assert");
const shiki_1 = require("shiki");
const array_1 = require("./array");
const JSX = __importStar(require("./jsx"));
const aliases = new Map();
for (const lang of shiki_1.BUNDLED_LANGUAGES) {
    for (const alias of lang.aliases || []) {
        aliases.set(alias, lang.id);
    }
}
const supportedLanguages = (0, array_1.unique)(["text", ...aliases.keys(), ...shiki_1.BUNDLED_LANGUAGES.map((lang) => lang.id)]).sort();
class DoubleHighlighter {
    constructor(highlighter, light, dark) {
        this.highlighter = highlighter;
        this.light = light;
        this.dark = dark;
        this.schemes = new Map();
    }
    highlight(code, lang) {
        const lightTokens = this.highlighter.codeToThemedTokens(code, lang, this.light, { includeExplanation: false });
        const darkTokens = this.highlighter.codeToThemedTokens(code, lang, this.dark, { includeExplanation: false });
        // If this fails... something went *very* wrong.
        (0, assert_1.ok)(lightTokens.length === darkTokens.length);
        const docEls = [];
        for (const [lightLine, darkLine] of (0, array_1.zip)(lightTokens, darkTokens)) {
            // Different themes can have different rules for when colors change... so unfortunately we have to deal with different
            // sets of tokens.Example: light_plus and dark_plus tokenize " = " differently in the `schemes`
            // declaration for this file.
            while (lightLine.length && darkLine.length) {
                // Simple case, same token.
                if (lightLine[0].content === darkLine[0].content) {
                    docEls.push(JSX.createElement("span", { class: this.getClass(lightLine[0].color, darkLine[0].color) }, lightLine[0].content));
                    lightLine.shift();
                    darkLine.shift();
                    continue;
                }
                if (lightLine[0].content.length < darkLine[0].content.length) {
                    docEls.push(JSX.createElement("span", { class: this.getClass(lightLine[0].color, darkLine[0].color) }, lightLine[0].content));
                    darkLine[0].content = darkLine[0].content.substring(lightLine[0].content.length);
                    lightLine.shift();
                    continue;
                }
                docEls.push(JSX.createElement("span", { class: this.getClass(lightLine[0].color, darkLine[0].color) }, darkLine[0].content));
                lightLine[0].content = lightLine[0].content.substring(darkLine[0].content.length);
                darkLine.shift();
            }
            docEls.push(JSX.createElement("br", null));
        }
        docEls.pop(); // Remove last <br>
        return JSX.renderElement(JSX.createElement(JSX.Fragment, null, docEls));
    }
    getStyles() {
        const style = [":root {"];
        const lightRules = [];
        const darkRules = [];
        let i = 0;
        for (const key of this.schemes.keys()) {
            const [light, dark] = key.split(" | ");
            style.push(`    --light-hl-${i}: ${light};`);
            style.push(`    --dark-hl-${i}: ${dark};`);
            lightRules.push(`    --hl-${i}: var(--light-hl-${i});`);
            darkRules.push(`    --hl-${i}: var(--dark-hl-${i});`);
            i++;
        }
        style.push(`    --light-code-background: ${this.highlighter.getTheme(this.light).bg};`);
        style.push(`    --dark-code-background: ${this.highlighter.getTheme(this.dark).bg};`);
        lightRules.push(`    --code-background: var(--light-code-background);`);
        darkRules.push(`    --code-background: var(--dark-code-background);`);
        style.push("}", "");
        style.push("@media (prefers-color-scheme: light) { :root {");
        style.push(...lightRules);
        style.push("} }", "");
        style.push("@media (prefers-color-scheme: dark) { :root {");
        style.push(...darkRules);
        style.push("} }", "");
        style.push(":root[data-theme='light'] {");
        style.push(...lightRules);
        style.push("}", "");
        style.push(":root[data-theme='dark'] {");
        style.push(...darkRules);
        style.push("}", "");
        for (i = 0; i < this.schemes.size; i++) {
            style.push(`.hl-${i} { color: var(--hl-${i}); }`);
        }
        style.push("pre, code { background: var(--code-background); }", "");
        return style.join("\n");
    }
    getClass(lightColor, darkColor) {
        const key = `${lightColor} | ${darkColor}`;
        let scheme = this.schemes.get(key);
        if (scheme == null) {
            scheme = `hl-${this.schemes.size}`;
            this.schemes.set(key, scheme);
        }
        return scheme;
    }
}
let highlighter;
async function loadHighlighter(lightTheme, darkTheme) {
    if (highlighter)
        return;
    const hl = await (0, shiki_1.getHighlighter)({ themes: [lightTheme, darkTheme] });
    highlighter = new DoubleHighlighter(hl, lightTheme, darkTheme);
}
exports.loadHighlighter = loadHighlighter;
function isSupportedLanguage(lang) {
    return getSupportedLanguages().includes(lang);
}
exports.isSupportedLanguage = isSupportedLanguage;
function getSupportedLanguages() {
    return supportedLanguages;
}
exports.getSupportedLanguages = getSupportedLanguages;
function highlight(code, lang) {
    (0, assert_1.ok)(highlighter, "Tried to highlight with an uninitialized highlighter");
    if (!isSupportedLanguage(lang)) {
        return code;
    }
    if (lang === "text") {
        return JSX.renderElement(JSX.createElement(JSX.Fragment, null, code));
    }
    return highlighter.highlight(code, aliases.get(lang) ?? lang);
}
exports.highlight = highlight;
function getStyles() {
    (0, assert_1.ok)(highlighter, "Tried to highlight with an uninitialized highlighter");
    return highlighter.getStyles();
}
exports.getStyles = getStyles;
