"use strict";
/* eslint-disable no-console */
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
const ExitCodes = {
    Ok: 0,
    OptionError: 1,
    CompileError: 3,
    ValidationError: 4,
    OutputError: 5,
    ExceptionThrown: 6,
};
const td = __importStar(require("typedoc"));
const app = new td.Application();
app.options.addReader(new td.ArgumentsReader(0));
app.options.addReader(new td.TypeDocReader());
app.options.addReader(new td.PackageJsonReader());
app.options.addReader(new td.TSConfigReader());
app.options.addReader(new td.ArgumentsReader(300));
void run(app)
    .catch((error) => {
    console.error("TypeDoc exiting with unexpected error:");
    console.error(error);
    if (app.options.getValue("skipErrorChecking")) {
        console.error("Try turning off --skipErrorChecking. If TypeDoc still crashes, please report a bug.");
    }
    return ExitCodes.ExceptionThrown;
})
    .then((exitCode) => {
    process.exitCode = exitCode;
});
async function run(app) {
    const start = Date.now();
    await app.bootstrapWithPlugins();
    if (app.options.getValue("version")) {
        console.log(app.toString());
        return ExitCodes.Ok;
    }
    if (app.options.getValue("help")) {
        console.log(app.options.getHelp());
        return ExitCodes.Ok;
    }
    if (app.options.getValue("showConfig")) {
        console.log(app.options.getRawValues());
        return ExitCodes.Ok;
    }
    if (app.logger.hasErrors()) {
        return ExitCodes.OptionError;
    }
    if (app.options.getValue("treatWarningsAsErrors") &&
        app.logger.hasWarnings()) {
        return ExitCodes.OptionError;
    }
    if (app.options.getValue("watch")) {
        app.convertAndWatch(async (project) => {
            const json = app.options.getValue("json");
            if (!json || app.options.isSet("out")) {
                await app.generateDocs(project, app.options.getValue("out"));
            }
            if (json) {
                await app.generateJson(project, json);
            }
        });
        return ExitCodes.Ok;
    }
    const project = app.convert();
    if (!project) {
        return ExitCodes.CompileError;
    }
    if (app.options.getValue("treatWarningsAsErrors") &&
        app.logger.hasWarnings()) {
        return ExitCodes.CompileError;
    }
    const preValidationWarnCount = app.logger.warningCount;
    app.validate(project);
    const hadValidationWarnings = app.logger.warningCount !== preValidationWarnCount;
    if (app.logger.hasErrors()) {
        return ExitCodes.ValidationError;
    }
    if (hadValidationWarnings &&
        (app.options.getValue("treatWarningsAsErrors") ||
            app.options.getValue("treatValidationWarningsAsErrors"))) {
        return ExitCodes.ValidationError;
    }
    if (app.options.getValue("emit") !== "none") {
        const json = app.options.getValue("json");
        if (!json || app.options.isSet("out")) {
            await app.generateDocs(project, app.options.getValue("out"));
        }
        if (json) {
            await app.generateJson(project, json);
        }
        if (app.logger.hasErrors()) {
            return ExitCodes.OutputError;
        }
        if (app.options.getValue("treatWarningsAsErrors") &&
            app.logger.hasWarnings()) {
            return ExitCodes.OutputError;
        }
    }
    app.logger.verbose(`Full run took ${Date.now() - start}ms`);
    return ExitCodes.Ok;
}
