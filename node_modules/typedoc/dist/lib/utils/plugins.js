"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.loadPlugins = void 0;
const path_1 = require("path");
const url_1 = require("url");
const paths_1 = require("./paths");
async function loadPlugins(app, plugins) {
    for (const plugin of plugins) {
        const pluginDisplay = getPluginDisplayName(plugin);
        try {
            // eslint-disable-next-line @typescript-eslint/no-var-requires
            let instance;
            try {
                instance = require(plugin);
            }
            catch (error) {
                if (error.code === "ERR_REQUIRE_ESM") {
                    // On Windows, we need to ensure this path is a file path.
                    // Or we'll get ERR_UNSUPPORTED_ESM_URL_SCHEME
                    const esmPath = (0, path_1.isAbsolute)(plugin)
                        ? (0, url_1.pathToFileURL)(plugin).toString()
                        : plugin;
                    instance = await import(esmPath);
                }
                else {
                    throw error;
                }
            }
            const initFunction = instance.load;
            if (typeof initFunction === "function") {
                await initFunction(app);
                app.logger.info(`Loaded plugin ${pluginDisplay}`);
            }
            else {
                app.logger.error(`Invalid structure in plugin ${pluginDisplay}, no load function found.`);
            }
        }
        catch (error) {
            app.logger.error(`The plugin ${pluginDisplay} could not be loaded.`);
            if (error instanceof Error && error.stack) {
                app.logger.error(error.stack);
            }
        }
    }
}
exports.loadPlugins = loadPlugins;
function getPluginDisplayName(plugin) {
    const path = (0, paths_1.nicePath)(plugin);
    if (path.startsWith("./node_modules/")) {
        return path.substring("./node_modules/".length);
    }
    return plugin;
}
