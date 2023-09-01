"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.analytics = void 0;
const utils_1 = require("../../../../utils");
function analytics(context) {
    const gaID = context.options.getValue("gaID");
    if (!gaID)
        return;
    const script = `
window.dataLayer = window.dataLayer || [];
function gtag(){dataLayer.push(arguments);}
gtag('js', new Date());
gtag('config', '${gaID}');
`.trim();
    return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
        utils_1.JSX.createElement("script", { async: true, src: "https://www.googletagmanager.com/gtag/js?id=" + gaID }),
        utils_1.JSX.createElement("script", null,
            utils_1.JSX.createElement(utils_1.JSX.Raw, { html: script }))));
}
exports.analytics = analytics;
