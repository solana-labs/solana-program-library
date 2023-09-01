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
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports, p)) __createBinding(exports, m, p);
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.TypeScript = exports.SerializeEvent = exports.Deserializer = exports.Serializer = exports.JSONOutput = exports.normalizePath = exports.MinimalSourceFile = exports.EventHooks = exports.EntryPointStrategy = exports.TypeDocReader = exports.TSConfigReader = exports.ParameterType = exports.ParameterHint = exports.PackageJsonReader = exports.Options = exports.Logger = exports.LogLevel = exports.JSX = exports.CommentStyle = exports.BindOption = exports.ArgumentsReader = exports.IndexEvent = exports.MarkdownEvent = exports.RendererEvent = exports.PageEvent = exports.Theme = exports.UrlMapping = exports.DefaultThemeRenderContext = exports.DefaultTheme = exports.Renderer = exports.Context = exports.Converter = exports.Models = exports.resetReflectionID = exports.Event = exports.EventDispatcher = exports.Application = void 0;
var application_1 = require("./lib/application");
Object.defineProperty(exports, "Application", { enumerable: true, get: function () { return application_1.Application; } });
var events_1 = require("./lib/utils/events");
Object.defineProperty(exports, "EventDispatcher", { enumerable: true, get: function () { return events_1.EventDispatcher; } });
Object.defineProperty(exports, "Event", { enumerable: true, get: function () { return events_1.Event; } });
var abstract_1 = require("./lib/models/reflections/abstract");
Object.defineProperty(exports, "resetReflectionID", { enumerable: true, get: function () { return abstract_1.resetReflectionID; } });
/**
 * All symbols documented under the Models namespace are also available in the root import.
 */
exports.Models = __importStar(require("./lib/models"));
__exportStar(require("./lib/models"), exports);
var converter_1 = require("./lib/converter");
Object.defineProperty(exports, "Converter", { enumerable: true, get: function () { return converter_1.Converter; } });
Object.defineProperty(exports, "Context", { enumerable: true, get: function () { return converter_1.Context; } });
var output_1 = require("./lib/output");
Object.defineProperty(exports, "Renderer", { enumerable: true, get: function () { return output_1.Renderer; } });
Object.defineProperty(exports, "DefaultTheme", { enumerable: true, get: function () { return output_1.DefaultTheme; } });
Object.defineProperty(exports, "DefaultThemeRenderContext", { enumerable: true, get: function () { return output_1.DefaultThemeRenderContext; } });
Object.defineProperty(exports, "UrlMapping", { enumerable: true, get: function () { return output_1.UrlMapping; } });
Object.defineProperty(exports, "Theme", { enumerable: true, get: function () { return output_1.Theme; } });
Object.defineProperty(exports, "PageEvent", { enumerable: true, get: function () { return output_1.PageEvent; } });
Object.defineProperty(exports, "RendererEvent", { enumerable: true, get: function () { return output_1.RendererEvent; } });
Object.defineProperty(exports, "MarkdownEvent", { enumerable: true, get: function () { return output_1.MarkdownEvent; } });
Object.defineProperty(exports, "IndexEvent", { enumerable: true, get: function () { return output_1.IndexEvent; } });
var utils_1 = require("./lib/utils");
Object.defineProperty(exports, "ArgumentsReader", { enumerable: true, get: function () { return utils_1.ArgumentsReader; } });
Object.defineProperty(exports, "BindOption", { enumerable: true, get: function () { return utils_1.BindOption; } });
Object.defineProperty(exports, "CommentStyle", { enumerable: true, get: function () { return utils_1.CommentStyle; } });
Object.defineProperty(exports, "JSX", { enumerable: true, get: function () { return utils_1.JSX; } });
Object.defineProperty(exports, "LogLevel", { enumerable: true, get: function () { return utils_1.LogLevel; } });
Object.defineProperty(exports, "Logger", { enumerable: true, get: function () { return utils_1.Logger; } });
Object.defineProperty(exports, "Options", { enumerable: true, get: function () { return utils_1.Options; } });
Object.defineProperty(exports, "PackageJsonReader", { enumerable: true, get: function () { return utils_1.PackageJsonReader; } });
Object.defineProperty(exports, "ParameterHint", { enumerable: true, get: function () { return utils_1.ParameterHint; } });
Object.defineProperty(exports, "ParameterType", { enumerable: true, get: function () { return utils_1.ParameterType; } });
Object.defineProperty(exports, "TSConfigReader", { enumerable: true, get: function () { return utils_1.TSConfigReader; } });
Object.defineProperty(exports, "TypeDocReader", { enumerable: true, get: function () { return utils_1.TypeDocReader; } });
Object.defineProperty(exports, "EntryPointStrategy", { enumerable: true, get: function () { return utils_1.EntryPointStrategy; } });
Object.defineProperty(exports, "EventHooks", { enumerable: true, get: function () { return utils_1.EventHooks; } });
Object.defineProperty(exports, "MinimalSourceFile", { enumerable: true, get: function () { return utils_1.MinimalSourceFile; } });
Object.defineProperty(exports, "normalizePath", { enumerable: true, get: function () { return utils_1.normalizePath; } });
var serialization_1 = require("./lib/serialization");
Object.defineProperty(exports, "JSONOutput", { enumerable: true, get: function () { return serialization_1.JSONOutput; } });
Object.defineProperty(exports, "Serializer", { enumerable: true, get: function () { return serialization_1.Serializer; } });
Object.defineProperty(exports, "Deserializer", { enumerable: true, get: function () { return serialization_1.Deserializer; } });
Object.defineProperty(exports, "SerializeEvent", { enumerable: true, get: function () { return serialization_1.SerializeEvent; } });
const typescript_1 = __importDefault(require("typescript"));
exports.TypeScript = typescript_1.default;
