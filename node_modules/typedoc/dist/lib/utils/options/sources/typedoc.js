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
exports.addTypeDocOptions = void 0;
const loggers_1 = require("../../loggers");
const declaration_1 = require("../declaration");
const shiki_1 = require("shiki");
const sort_1 = require("../../sort");
const entry_point_1 = require("../../entry-point");
const kind_1 = require("../../../models/reflections/kind");
const Validation = __importStar(require("../../validation"));
const tsdoc_defaults_1 = require("../tsdoc-defaults");
const enum_1 = require("../../enum");
// For convenience, added in the same order as they are documented on the website.
function addTypeDocOptions(options) {
    ///////////////////////////
    // Configuration Options //
    ///////////////////////////
    options.addDeclaration({
        type: declaration_1.ParameterType.Path,
        name: "options",
        help: "Specify a json option file that should be loaded. If not specified TypeDoc will look for 'typedoc.json' in the current directory.",
        hint: declaration_1.ParameterHint.File,
        defaultValue: "",
    });
    options.addDeclaration({
        type: declaration_1.ParameterType.Path,
        name: "tsconfig",
        help: "Specify a TypeScript config file that should be loaded. If not specified TypeDoc will look for 'tsconfig.json' in the current directory.",
        hint: declaration_1.ParameterHint.File,
        defaultValue: "",
    });
    options.addDeclaration({
        name: "compilerOptions",
        help: "Selectively override the TypeScript compiler options used by TypeDoc.",
        type: declaration_1.ParameterType.Mixed,
        configFileOnly: true,
        validate(value) {
            if (!Validation.validate({}, value)) {
                throw new Error("The 'compilerOptions' option must be a non-array object.");
            }
        },
    });
    ///////////////////////////
    ////// Input Options //////
    ///////////////////////////
    options.addDeclaration({
        name: "entryPoints",
        help: "The entry points of your documentation.",
        type: declaration_1.ParameterType.GlobArray,
    });
    options.addDeclaration({
        name: "entryPointStrategy",
        help: "The strategy to be used to convert entry points into documentation modules.",
        type: declaration_1.ParameterType.Map,
        map: entry_point_1.EntryPointStrategy,
        defaultValue: entry_point_1.EntryPointStrategy.Resolve,
    });
    options.addDeclaration({
        name: "exclude",
        help: "Define patterns to be excluded when expanding a directory that was specified as an entry point.",
        type: declaration_1.ParameterType.GlobArray,
    });
    options.addDeclaration({
        name: "externalPattern",
        help: "Define patterns for files that should be considered being external.",
        type: declaration_1.ParameterType.GlobArray,
        defaultValue: ["**/node_modules/**"],
    });
    options.addDeclaration({
        name: "excludeExternals",
        help: "Prevent externally resolved symbols from being documented.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "excludeNotDocumented",
        help: "Prevent symbols that are not explicitly documented from appearing in the results.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "excludeNotDocumentedKinds",
        help: "Specify the type of reflections that can be removed by excludeNotDocumented.",
        type: declaration_1.ParameterType.Array,
        validate(value) {
            const invalid = new Set(value);
            const valid = new Set((0, enum_1.getEnumKeys)(kind_1.ReflectionKind));
            for (const notPermitted of [
                kind_1.ReflectionKind.Project,
                kind_1.ReflectionKind.TypeLiteral,
                kind_1.ReflectionKind.TypeParameter,
                kind_1.ReflectionKind.Parameter,
                kind_1.ReflectionKind.ObjectLiteral,
            ]) {
                valid.delete(kind_1.ReflectionKind[notPermitted]);
            }
            for (const v of valid) {
                invalid.delete(v);
            }
            if (invalid.size !== 0) {
                throw new Error(`excludeNotDocumentedKinds may only specify known values, and invalid values were provided (${Array.from(invalid).join(", ")}). The valid kinds are:\n${Array.from(valid).join(", ")}`);
            }
        },
        defaultValue: [
            kind_1.ReflectionKind[kind_1.ReflectionKind.Module],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Namespace],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Enum],
            // Not including enum member here by default
            kind_1.ReflectionKind[kind_1.ReflectionKind.Variable],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Function],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Class],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Interface],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Constructor],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Property],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Method],
            kind_1.ReflectionKind[kind_1.ReflectionKind.CallSignature],
            kind_1.ReflectionKind[kind_1.ReflectionKind.IndexSignature],
            kind_1.ReflectionKind[kind_1.ReflectionKind.ConstructorSignature],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Accessor],
            kind_1.ReflectionKind[kind_1.ReflectionKind.GetSignature],
            kind_1.ReflectionKind[kind_1.ReflectionKind.SetSignature],
            kind_1.ReflectionKind[kind_1.ReflectionKind.TypeAlias],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Reference],
        ],
    });
    options.addDeclaration({
        name: "excludeInternal",
        help: "Prevent symbols that are marked with @internal from being documented.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "excludePrivate",
        help: "Ignore private variables and methods.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "excludeProtected",
        help: "Ignore protected variables and methods.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "excludeReferences",
        help: "If a symbol is exported multiple times, ignore all but the first export.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "externalSymbolLinkMappings",
        help: "Define custom links for symbols not included in the documentation.",
        type: declaration_1.ParameterType.Mixed,
        defaultValue: {},
        validate(value) {
            const error = "externalSymbolLinkMappings must be a Record<package name, Record<symbol name, link>>";
            if (!Validation.validate({}, value)) {
                throw new Error(error);
            }
            for (const mappings of Object.values(value)) {
                if (!Validation.validate({}, mappings)) {
                    throw new Error(error);
                }
                for (const link of Object.values(mappings)) {
                    if (typeof link !== "string") {
                        throw new Error(error);
                    }
                }
            }
        },
    });
    options.addDeclaration({
        name: "media",
        help: "Specify the location with media files that should be copied to the output directory.",
        type: declaration_1.ParameterType.Path,
        hint: declaration_1.ParameterHint.Directory,
    });
    options.addDeclaration({
        name: "includes",
        help: "Specify the location to look for included documents (use [[include:FILENAME]] in comments).",
        type: declaration_1.ParameterType.Path,
        hint: declaration_1.ParameterHint.Directory,
    });
    ///////////////////////////
    ///// Output Options //////
    ///////////////////////////
    options.addDeclaration({
        name: "out",
        help: "Specify the location the documentation should be written to.",
        type: declaration_1.ParameterType.Path,
        hint: declaration_1.ParameterHint.Directory,
        defaultValue: "./docs",
    });
    options.addDeclaration({
        name: "json",
        help: "Specify the location and filename a JSON file describing the project is written to.",
        type: declaration_1.ParameterType.Path,
        hint: declaration_1.ParameterHint.File,
    });
    options.addDeclaration({
        name: "pretty",
        help: "Specify whether the output JSON should be formatted with tabs.",
        type: declaration_1.ParameterType.Boolean,
        defaultValue: true,
    });
    options.addDeclaration({
        name: "emit",
        help: "Specify what TypeDoc should emit, 'docs', 'both', or 'none'.",
        type: declaration_1.ParameterType.Map,
        map: declaration_1.EmitStrategy,
        defaultValue: "docs",
    });
    options.addDeclaration({
        name: "theme",
        help: "Specify the theme name to render the documentation with",
        type: declaration_1.ParameterType.String,
        defaultValue: "default",
    });
    const defaultLightTheme = "light-plus";
    const defaultDarkTheme = "dark-plus";
    options.addDeclaration({
        name: "lightHighlightTheme",
        help: "Specify the code highlighting theme in light mode.",
        type: declaration_1.ParameterType.String,
        defaultValue: defaultLightTheme,
        validate(value) {
            if (!shiki_1.BUNDLED_THEMES.includes(value)) {
                throw new Error(`lightHighlightTheme must be one of the following: ${shiki_1.BUNDLED_THEMES.join(", ")}`);
            }
        },
    });
    options.addDeclaration({
        name: "darkHighlightTheme",
        help: "Specify the code highlighting theme in dark mode.",
        type: declaration_1.ParameterType.String,
        defaultValue: defaultDarkTheme,
        validate(value) {
            if (!shiki_1.BUNDLED_THEMES.includes(value)) {
                throw new Error(`darkHighlightTheme must be one of the following: ${shiki_1.BUNDLED_THEMES.join(", ")}`);
            }
        },
    });
    options.addDeclaration({
        name: "customCss",
        help: "Path to a custom CSS file to for the theme to import.",
        type: declaration_1.ParameterType.Path,
    });
    options.addDeclaration({
        name: "markedOptions",
        help: "Specify the options passed to Marked, the Markdown parser used by TypeDoc.",
        type: declaration_1.ParameterType.Mixed,
        configFileOnly: true,
        validate(value) {
            if (!Validation.validate({}, value)) {
                throw new Error("The 'markedOptions' option must be a non-array object.");
            }
        },
    });
    options.addDeclaration({
        name: "name",
        help: "Set the name of the project that will be used in the header of the template.",
    });
    options.addDeclaration({
        name: "includeVersion",
        help: "Add the package version to the project name.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "disableSources",
        help: "Disable setting the source of a reflection when documenting it.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "basePath",
        help: "Specifies the base path to be used when displaying file paths.",
        type: declaration_1.ParameterType.Path,
    });
    options.addDeclaration({
        name: "excludeTags",
        help: "Remove the listed block/modifier tags from doc comments.",
        type: declaration_1.ParameterType.Array,
        defaultValue: [
            "@override",
            "@virtual",
            "@privateRemarks",
            "@satisfies",
            "@overload",
        ],
        validate(value) {
            if (!Validation.validate([Array, Validation.isTagString], value)) {
                throw new Error(`excludeTags must be an array of valid tag names.`);
            }
        },
    });
    options.addDeclaration({
        name: "readme",
        help: "Path to the readme file that should be displayed on the index page. Pass `none` to disable the index page and start the documentation on the globals page.",
        type: declaration_1.ParameterType.Path,
    });
    options.addDeclaration({
        name: "cname",
        help: "Set the CNAME file text, it's useful for custom domains on GitHub Pages.",
    });
    options.addDeclaration({
        name: "sourceLinkTemplate",
        help: "Specify a link template to be used when generating source urls. If not set, will be automatically created using the git remote. Supports {path}, {line}, {gitRevision} placeholders.",
    });
    options.addDeclaration({
        name: "gitRevision",
        help: "Use specified revision instead of the last revision for linking to GitHub/Bitbucket source files.",
    });
    options.addDeclaration({
        name: "gitRemote",
        help: "Use the specified remote for linking to GitHub/Bitbucket source files.",
        defaultValue: "origin",
    });
    options.addDeclaration({
        name: "githubPages",
        help: "Generate a .nojekyll file to prevent 404 errors in GitHub Pages. Defaults to `true`.",
        type: declaration_1.ParameterType.Boolean,
        defaultValue: true,
    });
    options.addDeclaration({
        name: "htmlLang",
        help: "Sets the lang attribute in the generated html tag.",
        type: declaration_1.ParameterType.String,
        defaultValue: "en",
    });
    options.addDeclaration({
        name: "gaID",
        help: "Set the Google Analytics tracking ID and activate tracking code.",
    });
    options.addDeclaration({
        name: "hideGenerator",
        help: "Do not print the TypeDoc link at the end of the page.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "hideParameterTypesInTitle",
        help: "Hides parameter types in signature titles for easier scanning.",
        type: declaration_1.ParameterType.Boolean,
        defaultValue: true,
    });
    options.addDeclaration({
        name: "cacheBust",
        help: "Include the generation time in links to static assets.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "searchInComments",
        help: "If set, the search index will also include comments. This will greatly increase the size of the search index.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "cleanOutputDir",
        help: "If set, TypeDoc will remove the output directory before writing output.",
        type: declaration_1.ParameterType.Boolean,
        defaultValue: true,
    });
    options.addDeclaration({
        name: "titleLink",
        help: "Set the link the title in the header points to. Defaults to the documentation homepage.",
        type: declaration_1.ParameterType.String,
    });
    options.addDeclaration({
        name: "navigationLinks",
        help: "Defines links to be included in the header.",
        type: declaration_1.ParameterType.Mixed,
        defaultValue: {},
        validate(value) {
            if (!isObject(value)) {
                throw new Error(`navigationLinks must be an object with string labels as keys and URL values.`);
            }
            if (Object.values(value).some((x) => typeof x !== "string")) {
                throw new Error(`All values of navigationLinks must be string URLs.`);
            }
        },
    });
    options.addDeclaration({
        name: "sidebarLinks",
        help: "Defines links to be included in the sidebar.",
        type: declaration_1.ParameterType.Mixed,
        defaultValue: {},
        validate(value) {
            if (!isObject(value)) {
                throw new Error(`sidebarLinks must be an object with string labels as keys and URL values.`);
            }
            if (Object.values(value).some((x) => typeof x !== "string")) {
                throw new Error(`All values of sidebarLinks must be string URLs.`);
            }
        },
    });
    options.addDeclaration({
        name: "navigation",
        help: "Determines how the navigation sidebar is organized.",
        type: declaration_1.ParameterType.Flags,
        defaults: {
            includeCategories: false,
            includeGroups: false,
        },
    });
    options.addDeclaration({
        name: "visibilityFilters",
        help: "Specify the default visibility for builtin filters and additional filters according to modifier tags.",
        type: declaration_1.ParameterType.Mixed,
        configFileOnly: true,
        defaultValue: {
            protected: false,
            private: false,
            inherited: true,
            external: false,
        },
        validate(value) {
            const knownKeys = ["protected", "private", "inherited", "external"];
            if (!value || typeof value !== "object") {
                throw new Error("visibilityFilters must be an object.");
            }
            for (const [key, val] of Object.entries(value)) {
                if (!key.startsWith("@") && !knownKeys.includes(key)) {
                    throw new Error(`visibilityFilters can only include the following non-@ keys: ${knownKeys.join(", ")}`);
                }
                if (typeof val !== "boolean") {
                    throw new Error(`All values of visibilityFilters must be booleans.`);
                }
            }
        },
    });
    options.addDeclaration({
        name: "searchCategoryBoosts",
        help: "Configure search to give a relevance boost to selected categories",
        type: declaration_1.ParameterType.Mixed,
        configFileOnly: true,
        defaultValue: {},
        validate(value) {
            if (!isObject(value)) {
                throw new Error("The 'searchCategoryBoosts' option must be a non-array object.");
            }
            if (Object.values(value).some((x) => typeof x !== "number")) {
                throw new Error("All values of 'searchCategoryBoosts' must be numbers.");
            }
        },
    });
    options.addDeclaration({
        name: "searchGroupBoosts",
        help: 'Configure search to give a relevance boost to selected kinds (eg "class")',
        type: declaration_1.ParameterType.Mixed,
        configFileOnly: true,
        defaultValue: {},
        validate(value) {
            if (!isObject(value)) {
                throw new Error("The 'searchGroupBoosts' option must be a non-array object.");
            }
            if (Object.values(value).some((x) => typeof x !== "number")) {
                throw new Error("All values of 'searchGroupBoosts' must be numbers.");
            }
        },
    });
    ///////////////////////////
    ///// Comment Options /////
    ///////////////////////////
    options.addDeclaration({
        name: "jsDocCompatibility",
        help: "Sets compatibility options for comment parsing that increase similarity with JSDoc comments.",
        type: declaration_1.ParameterType.Flags,
        defaults: {
            defaultTag: true,
            exampleTag: true,
            inheritDocTag: true,
            ignoreUnescapedBraces: true,
        },
    });
    options.addDeclaration({
        name: "commentStyle",
        help: "Determines how TypeDoc searches for comments.",
        type: declaration_1.ParameterType.Map,
        map: declaration_1.CommentStyle,
        defaultValue: declaration_1.CommentStyle.JSDoc,
    });
    options.addDeclaration({
        name: "useTsLinkResolution",
        help: "Use TypeScript's link resolution when determining where @link tags point. This only applies to JSDoc style comments.",
        type: declaration_1.ParameterType.Boolean,
        defaultValue: true,
    });
    options.addDeclaration({
        name: "blockTags",
        help: "Block tags which TypeDoc should recognize when parsing comments.",
        type: declaration_1.ParameterType.Array,
        defaultValue: tsdoc_defaults_1.blockTags,
        validate(value) {
            if (!Validation.validate([Array, Validation.isTagString], value)) {
                throw new Error(`blockTags must be an array of valid tag names.`);
            }
        },
    });
    options.addDeclaration({
        name: "inlineTags",
        help: "Inline tags which TypeDoc should recognize when parsing comments.",
        type: declaration_1.ParameterType.Array,
        defaultValue: tsdoc_defaults_1.inlineTags,
        validate(value) {
            if (!Validation.validate([Array, Validation.isTagString], value)) {
                throw new Error(`inlineTags must be an array of valid tag names.`);
            }
        },
    });
    options.addDeclaration({
        name: "modifierTags",
        help: "Modifier tags which TypeDoc should recognize when parsing comments.",
        type: declaration_1.ParameterType.Array,
        defaultValue: tsdoc_defaults_1.modifierTags,
        validate(value) {
            if (!Validation.validate([Array, Validation.isTagString], value)) {
                throw new Error(`modifierTags must be an array of valid tag names.`);
            }
        },
    });
    ///////////////////////////
    // Organization Options ///
    ///////////////////////////
    options.addDeclaration({
        name: "categorizeByGroup",
        help: "Specify whether categorization will be done at the group level.",
        type: declaration_1.ParameterType.Boolean,
        defaultValue: true, // 0.25, change this to false.
    });
    options.addDeclaration({
        name: "defaultCategory",
        help: "Specify the default category for reflections without a category.",
        defaultValue: "Other",
    });
    options.addDeclaration({
        name: "categoryOrder",
        help: "Specify the order in which categories appear. * indicates the relative order for categories not in the list.",
        type: declaration_1.ParameterType.Array,
    });
    options.addDeclaration({
        name: "groupOrder",
        help: "Specify the order in which groups appear. * indicates the relative order for groups not in the list.",
        type: declaration_1.ParameterType.Array,
        // Defaults to the same as the defaultKindSortOrder in sort.ts
        defaultValue: [
            kind_1.ReflectionKind.Reference,
            // project is never a child so never added to a group
            kind_1.ReflectionKind.Module,
            kind_1.ReflectionKind.Namespace,
            kind_1.ReflectionKind.Enum,
            kind_1.ReflectionKind.EnumMember,
            kind_1.ReflectionKind.Class,
            kind_1.ReflectionKind.Interface,
            kind_1.ReflectionKind.TypeAlias,
            kind_1.ReflectionKind.Constructor,
            kind_1.ReflectionKind.Property,
            kind_1.ReflectionKind.Variable,
            kind_1.ReflectionKind.Function,
            kind_1.ReflectionKind.Accessor,
            kind_1.ReflectionKind.Method,
            // others are never added to groups
        ].map(kind_1.ReflectionKind.pluralString),
    });
    options.addDeclaration({
        name: "sort",
        help: "Specify the sort strategy for documented values.",
        type: declaration_1.ParameterType.Array,
        defaultValue: ["kind", "instance-first", "alphabetical"],
        validate(value) {
            const invalid = new Set(value);
            for (const v of sort_1.SORT_STRATEGIES) {
                invalid.delete(v);
            }
            if (invalid.size !== 0) {
                throw new Error(`sort may only specify known values, and invalid values were provided (${Array.from(invalid).join(", ")}). The valid sort strategies are:\n${sort_1.SORT_STRATEGIES.join(", ")}`);
            }
        },
    });
    options.addDeclaration({
        name: "kindSortOrder",
        help: "Specify the sort order for reflections when 'kind' is specified.",
        type: declaration_1.ParameterType.Array,
        defaultValue: [],
        validate(value) {
            const invalid = new Set(value);
            const valid = (0, enum_1.getEnumKeys)(kind_1.ReflectionKind);
            for (const v of valid) {
                invalid.delete(v);
            }
            if (invalid.size !== 0) {
                throw new Error(`kindSortOrder may only specify known values, and invalid values were provided (${Array.from(invalid).join(", ")}). The valid kinds are:\n${valid.join(", ")}`);
            }
        },
    });
    ///////////////////////////
    ///// General Options /////
    ///////////////////////////
    options.addDeclaration({
        name: "watch",
        help: "Watch files for changes and rebuild docs on change.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "preserveWatchOutput",
        help: "If set, TypeDoc will not clear the screen between compilation runs.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "skipErrorChecking",
        help: "Do not run TypeScript's type checking before generating docs.",
        type: declaration_1.ParameterType.Boolean,
        defaultValue: false,
    });
    options.addDeclaration({
        name: "help",
        help: "Print this message.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "version",
        help: "Print TypeDoc's version.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "showConfig",
        help: "Print the resolved configuration and exit.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "plugin",
        help: "Specify the npm plugins that should be loaded. Omit to load all installed plugins.",
        type: declaration_1.ParameterType.ModuleArray,
    });
    options.addDeclaration({
        name: "logLevel",
        help: "Specify what level of logging should be used.",
        type: declaration_1.ParameterType.Map,
        map: loggers_1.LogLevel,
        defaultValue: loggers_1.LogLevel.Info,
    });
    options.addDeclaration({
        name: "treatWarningsAsErrors",
        help: "If set, all warnings will be treated as errors.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "treatValidationWarningsAsErrors",
        help: "If set, warnings emitted during validation will be treated as errors. This option cannot be used to disable treatWarningsAsErrors for validation warnings.",
        type: declaration_1.ParameterType.Boolean,
    });
    options.addDeclaration({
        name: "intentionallyNotExported",
        help: "A list of types which should not produce 'referenced but not documented' warnings.",
        type: declaration_1.ParameterType.Array,
    });
    options.addDeclaration({
        name: "requiredToBeDocumented",
        help: "A list of reflection kinds that must be documented",
        type: declaration_1.ParameterType.Array,
        validate(values) {
            // this is good enough because the values of the ReflectionKind enum are all numbers
            const validValues = (0, enum_1.getEnumKeys)(kind_1.ReflectionKind);
            for (const kind of values) {
                if (!validValues.includes(kind)) {
                    throw new Error(`'${kind}' is an invalid value for 'requiredToBeDocumented'. Must be one of: ${validValues.join(", ")}`);
                }
            }
        },
        defaultValue: [
            kind_1.ReflectionKind[kind_1.ReflectionKind.Enum],
            kind_1.ReflectionKind[kind_1.ReflectionKind.EnumMember],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Variable],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Function],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Class],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Interface],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Property],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Method],
            kind_1.ReflectionKind[kind_1.ReflectionKind.Accessor],
            kind_1.ReflectionKind[kind_1.ReflectionKind.TypeAlias],
        ],
    });
    options.addDeclaration({
        name: "validation",
        help: "Specify which validation steps TypeDoc should perform on your generated documentation.",
        type: declaration_1.ParameterType.Flags,
        defaults: {
            notExported: true,
            invalidLink: true,
            notDocumented: false,
        },
    });
}
exports.addTypeDocOptions = addTypeDocOptions;
function isObject(x) {
    return !!x && typeof x === "object" && !Array.isArray(x);
}
