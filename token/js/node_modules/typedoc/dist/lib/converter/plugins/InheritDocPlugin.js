"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.InheritDocPlugin = void 0;
const models_1 = require("../../models");
const components_1 = require("../components");
const converter_1 = require("../converter");
const utils_1 = require("../../utils");
const array_1 = require("../../utils/array");
const declarationReference_1 = require("../comments/declarationReference");
const declarationReferenceResolver_1 = require("../comments/declarationReferenceResolver");
const application_events_1 = require("../../application-events");
/**
 * A plugin that handles `@inheritDoc` tags by copying documentation from another API item.
 * It is NOT responsible for handling bare JSDoc style `@inheritDoc` tags which do not specify
 * a target to inherit from. Those are handled by the ImplementsPlugin class.
 *
 * What gets copied:
 * - short text
 * - text
 * - `@remarks` block
 * - `@params` block
 * - `@typeParam` block
 * - `@return` block
 */
let InheritDocPlugin = class InheritDocPlugin extends components_1.ConverterComponent {
    constructor() {
        super(...arguments);
        // Key is depended on by Values
        this.dependencies = new utils_1.DefaultMap(() => []);
    }
    /**
     * Create a new InheritDocPlugin instance.
     */
    initialize() {
        this.owner.on(converter_1.Converter.EVENT_RESOLVE_END, (context) => this.processInheritDoc(context.project));
        this.owner.on(application_events_1.ApplicationEvents.REVIVE, this.processInheritDoc, this);
    }
    /**
     * Traverse through reflection descendant to check for `inheritDoc` tag.
     * If encountered, the parameter of the tag is used to determine a source reflection
     * that will provide actual comment.
     */
    processInheritDoc(project) {
        for (const reflection of Object.values(project.reflections)) {
            const source = extractInheritDocTagReference(reflection);
            if (!source)
                continue;
            const declRef = (0, declarationReference_1.parseDeclarationReference)(source, 0, source.length);
            if (!declRef || /\S/.test(source.substring(declRef[1]))) {
                this.application.logger.warn(`Declaration reference in @inheritDoc for ${reflection.getFriendlyFullName()} was not fully parsed and may resolve incorrectly.`);
            }
            let sourceRefl = declRef && (0, declarationReferenceResolver_1.resolveDeclarationReference)(reflection, declRef[0]);
            if (reflection instanceof models_1.SignatureReflection) {
                // Assumes that if there are overloads, they are declared in the same order as the parent.
                // TS doesn't check this, but if a user messes this up then they are almost
                // guaranteed to run into bugs where they can't call a method on a child class
                // but if they assign (without a type assertion) that child to a variable of the parent class
                // then they can call the method.
                if (sourceRefl instanceof models_1.DeclarationReflection) {
                    const index = reflection.parent
                        .getAllSignatures()
                        .indexOf(reflection);
                    sourceRefl =
                        sourceRefl.getAllSignatures()[index] || sourceRefl;
                }
            }
            if (sourceRefl instanceof models_1.DeclarationReflection &&
                sourceRefl.kindOf(models_1.ReflectionKind.Accessor)) {
                // Accessors, like functions, never have comments on their actual root reflection.
                // If the user didn't specify whether to inherit from the getter or setter, then implicitly
                // try to inherit from the getter, #1968.
                sourceRefl = sourceRefl.getSignature || sourceRefl.setSignature;
            }
            if (!sourceRefl) {
                this.application.logger.warn(`Failed to find "${source}" to inherit the comment from in the comment for ${reflection.getFullName()}`);
                continue;
            }
            this.copyComment(sourceRefl, reflection);
        }
        this.createCircularDependencyWarnings();
        this.dependencies.clear();
    }
    copyComment(source, target) {
        if (!target.comment)
            return;
        if (!source.comment &&
            source instanceof models_1.DeclarationReflection &&
            source.signatures) {
            source = source.signatures[0];
        }
        if (!source.comment &&
            source instanceof models_1.DeclarationReflection &&
            source.type instanceof models_1.ReflectionType &&
            source.type.declaration.signatures) {
            source = source.type.declaration.signatures[0];
        }
        if (!source.comment) {
            this.application.logger.warn(`${target.getFullName()} tried to copy a comment from ${source.getFullName()} with @inheritDoc, but the source has no associated comment.`);
            return;
        }
        // If the source also has a @inheritDoc tag, we can't do anything yet.
        // We'll try again later, once we've resolved the source's @inheritDoc reference.
        if (extractInheritDocTagReference(source)) {
            this.dependencies.get(source).push(target);
            return;
        }
        target.comment.removeTags("@inheritDoc");
        target.comment.summary = models_1.Comment.cloneDisplayParts(source.comment.summary);
        const remarks = source.comment.getTag("@remarks");
        if (remarks) {
            target.comment.blockTags.unshift(remarks.clone());
        }
        const returns = source.comment.getTag("@returns");
        if (returns) {
            target.comment.blockTags.push(returns.clone());
        }
        if (source instanceof models_1.SignatureReflection &&
            target instanceof models_1.SignatureReflection) {
            copySummaries(source.parameters, target.parameters);
            copySummaries(source.typeParameters, target.typeParameters);
        }
        else if (source instanceof models_1.DeclarationReflection &&
            target instanceof models_1.DeclarationReflection) {
            copySummaries(source.typeParameters, target.typeParameters);
        }
        // Now copy the comment for anyone who depends on me.
        const dependent = this.dependencies.get(target);
        this.dependencies.delete(target);
        for (const target2 of dependent) {
            this.copyComment(target, target2);
        }
    }
    createCircularDependencyWarnings() {
        const unwarned = new Set(this.dependencies.keys());
        const generateWarning = (orig) => {
            const parts = [orig.name];
            unwarned.delete(orig);
            let work = orig;
            do {
                work = this.dependencies.get(work)[0];
                unwarned.delete(work);
                parts.push(work.name);
            } while (!this.dependencies.get(work).includes(orig));
            parts.push(orig.name);
            this.application.logger.warn(`@inheritDoc specifies a circular inheritance chain: ${parts
                .reverse()
                .join(" -> ")}`);
        };
        for (const orig of this.dependencies.keys()) {
            if (unwarned.has(orig)) {
                generateWarning(orig);
            }
        }
    }
};
InheritDocPlugin = __decorate([
    (0, components_1.Component)({ name: "inheritDoc" })
], InheritDocPlugin);
exports.InheritDocPlugin = InheritDocPlugin;
function copySummaries(source, target) {
    for (const [s, t] of (0, array_1.zip)(source || [], target || [])) {
        t.comment = new models_1.Comment(s.comment?.summary);
    }
}
function extractInheritDocTagReference(reflection) {
    const comment = reflection.comment;
    if (!comment)
        return;
    const blockTag = comment.blockTags.find((tag) => tag.tag === "@inheritDoc");
    if (blockTag) {
        return blockTag.name;
    }
    const inlineTag = comment.summary.find((part) => part.kind === "inline-tag" && part.tag === "@inheritDoc");
    if (inlineTag) {
        return inlineTag.text;
    }
}
