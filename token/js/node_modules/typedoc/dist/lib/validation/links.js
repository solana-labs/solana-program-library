"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.validateLinks = void 0;
const linkTags = ["@link", "@linkcode", "@linkplain"];
function getBrokenLinks(comment) {
    const links = [];
    function processPart(part) {
        if (part.kind === "inline-tag" &&
            linkTags.includes(part.tag) &&
            !part.target) {
            links.push(part.text);
        }
    }
    comment?.summary.forEach(processPart);
    comment?.blockTags.forEach((tag) => tag.content.forEach(processPart));
    return links;
}
function validateLinks(project, logger) {
    for (const reflection of Object.values(project.reflections)) {
        for (const broken of getBrokenLinks(reflection.comment)) {
            logger.warn(`Failed to resolve link to "${broken}" in comment for ${reflection.getFriendlyFullName()}.`);
        }
    }
}
exports.validateLinks = validateLinks;
