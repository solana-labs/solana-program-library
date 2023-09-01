"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.SourceReference = void 0;
/**
 * Represents references of reflections to their defining source files.
 *
 * @see {@link DeclarationReflection.sources}
 */
class SourceReference {
    constructor(fileName, line, character) {
        this.fileName = fileName;
        this.fullFileName = fileName;
        this.line = line;
        this.character = character;
    }
    toObject() {
        return {
            fileName: this.fileName,
            line: this.line,
            character: this.character,
            url: this.url,
        };
    }
    fromObject(_de, obj) {
        this.url = obj.url;
    }
}
exports.SourceReference = SourceReference;
