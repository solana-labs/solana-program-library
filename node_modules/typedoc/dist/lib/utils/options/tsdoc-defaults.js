"use strict";
// If updating these lists, also see .config/typedoc-default.tsdoc.json
Object.defineProperty(exports, "__esModule", { value: true });
exports.modifierTags = exports.tsdocModifierTags = exports.inlineTags = exports.tsdocInlineTags = exports.blockTags = exports.tsdocBlockTags = void 0;
exports.tsdocBlockTags = [
    "@deprecated",
    "@param",
    "@remarks",
    "@returns",
    "@throws",
    "@privateRemarks",
    "@defaultValue",
    "@typeParam",
];
exports.blockTags = [
    ...exports.tsdocBlockTags,
    "@module",
    "@inheritDoc",
    "@group",
    "@category",
    // Alias for @typeParam
    "@template",
    // Because TypeScript is important!
    "@type",
    "@typedef",
    "@callback",
    "@prop",
    "@property",
    "@satisfies",
];
exports.tsdocInlineTags = ["@link", "@inheritDoc", "@label"];
exports.inlineTags = [...exports.tsdocInlineTags, "@linkcode", "@linkplain"];
exports.tsdocModifierTags = [
    "@public",
    "@private",
    "@protected",
    "@internal",
    "@readonly",
    "@packageDocumentation",
    "@eventProperty",
    "@alpha",
    "@beta",
    "@experimental",
    "@sealed",
    "@override",
    "@virtual",
];
exports.modifierTags = [
    ...exports.tsdocModifierTags,
    "@hidden",
    "@ignore",
    "@enum",
    "@event",
    "@overload",
    "@namespace",
    "@interface",
];
