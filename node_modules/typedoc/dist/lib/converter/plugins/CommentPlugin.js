"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.CommentPlugin = void 0;
const components_1 = require("../components");
const converter_1 = require("../converter");
const models_1 = require("../../models");
const utils_1 = require("../../utils");
/**
 * These tags are not useful to display in the generated documentation.
 * They should be ignored when parsing comments. Any relevant type information
 * (for JS users) will be consumed by TypeScript and need not be preserved
 * in the comment.
 *
 * Note that param/arg/argument/return/returns are not present.
 * These tags will have their type information stripped when parsing, but still
 * provide useful information for documentation.
 */
const NEVER_RENDERED = [
    "@augments",
    "@callback",
    "@class",
    "@constructor",
    "@enum",
    "@extends",
    "@this",
    "@type",
    "@typedef",
];
/**
 * Handles most behavior triggered by comments. `@group` and `@category` are handled by their respective plugins, but everything else is here.
 *
 * How it works today
 * ==================
 * During conversion:
 * - Handle visibility flags (`@private`, `@protected`. `@public`)
 * - Handle module renames (`@module`)
 * - Remove excluded tags & comment discovery tags (`@module`, `@packageDocumentation`)
 * - Copy comments for type parameters from the parent container (for classes/interfaces)
 *
 * Resolve begin:
 * - Remove hidden reflections
 *
 * Resolve:
 * - Apply `@label` tag
 * - Copy comments on signature containers to the signature if signatures don't already have a comment
 *   and then remove the comment on the container.
 * - Copy comments to parameters and type parameters (for signatures)
 * - Apply `@group` and `@category` tags
 *
 * Resolve end:
 * - Copy auto inherited comments from heritage clauses
 * - Handle `@inheritDoc`
 * - Resolve `@link` tags to point to target reflections
 *
 * How it should work
 * ==================
 * During conversion:
 * - Handle visibility flags (`@private`, `@protected`. `@public`)
 * - Handle module renames (`@module`)
 * - Remove excluded tags & comment discovery tags (`@module`, `@packageDocumentation`)
 *
 * Resolve begin (100):
 * - Copy auto inherited comments from heritage clauses
 * - Apply `@label` tag
 *
 * Resolve begin (75)
 * - Handle `@inheritDoc`
 *
 * Resolve begin (50)
 * - Copy comments on signature containers to the signature if signatures don't already have a comment
 *   and then remove the comment on the container.
 * - Copy comments for type parameters from the parent container (for classes/interfaces)
 *
 * Resolve begin (25)
 * - Remove hidden reflections
 *
 * Resolve:
 * - Copy comments to parameters and type parameters (for signatures)
 * - Apply `@group` and `@category` tags
 *
 * Resolve end:
 * - Resolve `@link` tags to point to target reflections
 *
 */
let CommentPlugin = class CommentPlugin extends components_1.ConverterComponent {
    get excludeNotDocumentedKinds() {
        this._excludeKinds ?? (this._excludeKinds = this.application.options
            .getValue("excludeNotDocumentedKinds")
            .reduce((a, b) => a | models_1.ReflectionKind[b], 0));
        return this._excludeKinds;
    }
    /**
     * Create a new CommentPlugin instance.
     */
    initialize() {
        this.listenTo(this.owner, {
            [converter_1.Converter.EVENT_CREATE_DECLARATION]: this.onDeclaration,
            [converter_1.Converter.EVENT_CREATE_SIGNATURE]: this.onDeclaration,
            [converter_1.Converter.EVENT_CREATE_TYPE_PARAMETER]: this.onCreateTypeParameter,
            [converter_1.Converter.EVENT_RESOLVE_BEGIN]: this.onBeginResolve,
            [converter_1.Converter.EVENT_RESOLVE]: this.onResolve,
            [converter_1.Converter.EVENT_END]: () => {
                this._excludeKinds = undefined;
            },
        });
    }
    /**
     * Apply all comment tag modifiers to the given reflection.
     *
     * @param reflection  The reflection the modifiers should be applied to.
     * @param comment  The comment that should be searched for modifiers.
     */
    applyModifiers(reflection, comment) {
        if (reflection.kindOf(models_1.ReflectionKind.SomeModule)) {
            comment.removeModifier("@namespace");
        }
        if (reflection.kindOf(models_1.ReflectionKind.Interface)) {
            comment.removeModifier("@interface");
        }
        if (comment.hasModifier("@private")) {
            reflection.setFlag(models_1.ReflectionFlag.Private);
            if (reflection.kindOf(models_1.ReflectionKind.CallSignature)) {
                reflection.parent?.setFlag(models_1.ReflectionFlag.Private);
            }
            comment.removeModifier("@private");
        }
        if (comment.hasModifier("@protected")) {
            reflection.setFlag(models_1.ReflectionFlag.Protected);
            if (reflection.kindOf(models_1.ReflectionKind.CallSignature)) {
                reflection.parent?.setFlag(models_1.ReflectionFlag.Protected);
            }
            comment.removeModifier("@protected");
        }
        if (comment.hasModifier("@public")) {
            reflection.setFlag(models_1.ReflectionFlag.Public);
            if (reflection.kindOf(models_1.ReflectionKind.CallSignature)) {
                reflection.parent?.setFlag(models_1.ReflectionFlag.Public);
            }
            comment.removeModifier("@public");
        }
        if (comment.hasModifier("@readonly")) {
            const target = reflection.kindOf(models_1.ReflectionKind.GetSignature)
                ? reflection.parent
                : reflection;
            target.setFlag(models_1.ReflectionFlag.Readonly);
            comment.removeModifier("@readonly");
        }
        if (comment.hasModifier("@event") ||
            comment.hasModifier("@eventProperty")) {
            comment.blockTags.push(new models_1.CommentTag("@group", [{ kind: "text", text: "Events" }]));
            comment.removeModifier("@event");
            comment.removeModifier("@eventProperty");
        }
        if (reflection.kindOf(models_1.ReflectionKind.Module | models_1.ReflectionKind.Namespace) ||
            reflection.kind === models_1.ReflectionKind.Project) {
            comment.removeTags("@module");
            comment.removeModifier("@packageDocumentation");
        }
    }
    /**
     * Triggered when the converter has created a type parameter reflection.
     *
     * @param context  The context object describing the current state the converter is in.
     * @param reflection  The reflection that is currently processed.
     */
    onCreateTypeParameter(_context, reflection) {
        const comment = reflection.parent?.comment;
        if (comment) {
            let tag = comment.getIdentifiedTag(reflection.name, "@typeParam");
            if (!tag) {
                tag = comment.getIdentifiedTag(reflection.name, "@template");
            }
            if (!tag) {
                tag = comment.getIdentifiedTag(`<${reflection.name}>`, "@param");
            }
            if (!tag) {
                tag = comment.getIdentifiedTag(reflection.name, "@param");
            }
            if (tag) {
                reflection.comment = new models_1.Comment(tag.content);
                (0, utils_1.removeIfPresent)(comment.blockTags, tag);
            }
        }
    }
    /**
     * Triggered when the converter has created a declaration or signature reflection.
     *
     * Invokes the comment parser.
     *
     * @param context  The context object describing the current state the converter is in.
     * @param reflection  The reflection that is currently processed.
     * @param node  The node that is currently processed if available.
     */
    onDeclaration(_context, reflection) {
        const comment = reflection.comment;
        if (!comment)
            return;
        if (reflection.kindOf(models_1.ReflectionKind.Module)) {
            const tag = comment.getTag("@module");
            if (tag) {
                // If no name is specified, this is a flag to mark a comment as a module comment
                // and should not result in a reflection rename.
                const newName = models_1.Comment.combineDisplayParts(tag.content).trim();
                if (newName.length && !newName.includes("\n")) {
                    reflection.name = newName;
                }
                (0, utils_1.removeIfPresent)(comment.blockTags, tag);
            }
        }
        this.applyModifiers(reflection, comment);
        this.removeExcludedTags(comment);
    }
    /**
     * Triggered when the converter begins resolving a project.
     *
     * @param context  The context object describing the current state the converter is in.
     */
    onBeginResolve(context) {
        const project = context.project;
        const reflections = Object.values(project.reflections);
        // Remove hidden reflections
        const hidden = new Set();
        for (const ref of reflections) {
            if (ref.kindOf(models_1.ReflectionKind.Accessor) && ref.flags.isReadonly) {
                const decl = ref;
                if (decl.setSignature) {
                    hidden.add(decl.setSignature);
                }
                // Clear flag set by @readonly since it shouldn't be rendered.
                ref.setFlag(models_1.ReflectionFlag.Readonly, false);
            }
            if (this.isHidden(ref)) {
                hidden.add(ref);
            }
        }
        hidden.forEach((reflection) => project.removeReflection(reflection));
        // remove functions with empty signatures after their signatures have been removed
        const [allRemoved, someRemoved] = (0, utils_1.partition)((0, utils_1.unique)((0, utils_1.filterMap)(hidden, (reflection) => reflection.parent?.kindOf(models_1.ReflectionKind.SignatureContainer)
            ? reflection.parent
            : void 0)), (method) => method.getNonIndexSignatures().length === 0);
        allRemoved.forEach((reflection) => {
            project.removeReflection(reflection);
        });
        someRemoved.forEach((reflection) => {
            reflection.sources = reflection
                .getNonIndexSignatures()
                .flatMap((s) => s.sources ?? []);
        });
    }
    /**
     * Triggered when the converter resolves a reflection.
     *
     * Cleans up comment tags related to signatures like `@param` or `@returns`
     * and moves their data to the corresponding parameter reflections.
     *
     * This hook also copies over the comment of function implementations to their
     * signatures.
     *
     * @param context  The context object describing the current state the converter is in.
     * @param reflection  The reflection that is currently resolved.
     */
    onResolve(context, reflection) {
        if (reflection.comment) {
            if (reflection.comment.label &&
                !/[A-Z_][A-Z0-9_]/.test(reflection.comment.label)) {
                context.logger.warn(`The label "${reflection.comment.label}" for ${reflection.getFriendlyFullName()} cannot be referenced with a declaration reference. ` +
                    `Labels may only contain A-Z, 0-9, and _, and may not start with a number.`);
            }
            mergeSeeTags(reflection.comment);
            movePropertyTags(reflection.comment, reflection);
        }
        if (!(reflection instanceof models_1.DeclarationReflection)) {
            return;
        }
        if (reflection.type instanceof models_1.ReflectionType) {
            this.moveCommentToSignatures(reflection, reflection.type.declaration.getNonIndexSignatures());
        }
        else {
            this.moveCommentToSignatures(reflection, reflection.getNonIndexSignatures());
        }
    }
    moveCommentToSignatures(reflection, signatures) {
        if (!signatures.length) {
            return;
        }
        const comment = reflection.comment;
        // Since this reflection has signatures, remove the comment from the parent
        // reflection. This is important so that in type aliases we don't end up with
        // a comment rendered twice.
        delete reflection.comment;
        for (const signature of signatures) {
            const childComment = (signature.comment || (signature.comment = comment?.clone()));
            if (!childComment)
                continue;
            signature.parameters?.forEach((parameter, index) => {
                if (parameter.name === "__namedParameters") {
                    const commentParams = childComment.blockTags.filter((tag) => tag.tag === "@param" && !tag.name?.includes("."));
                    if (signature.parameters?.length === commentParams.length &&
                        commentParams[index].name) {
                        parameter.name = commentParams[index].name;
                    }
                }
                moveNestedParamTags(childComment, parameter);
                const tag = childComment.getIdentifiedTag(parameter.name, "@param");
                if (tag) {
                    parameter.comment = new models_1.Comment(models_1.Comment.cloneDisplayParts(tag.content));
                }
            });
            for (const parameter of signature.typeParameters || []) {
                const tag = childComment.getIdentifiedTag(parameter.name, "@typeParam") ||
                    childComment.getIdentifiedTag(parameter.name, "@template") ||
                    childComment.getIdentifiedTag(`<${parameter.name}>`, "@param");
                if (tag) {
                    parameter.comment = new models_1.Comment(models_1.Comment.cloneDisplayParts(tag.content));
                }
            }
            childComment?.removeTags("@param");
            childComment?.removeTags("@typeParam");
            childComment?.removeTags("@template");
        }
    }
    removeExcludedTags(comment) {
        for (const tag of NEVER_RENDERED) {
            comment.removeTags(tag);
            comment.removeModifier(tag);
        }
        for (const tag of this.excludeTags) {
            comment.removeTags(tag);
            comment.removeModifier(tag);
        }
    }
    /**
     * Determines whether or not a reflection has been hidden
     *
     * @param reflection Reflection to check if hidden
     */
    isHidden(reflection) {
        const comment = reflection.comment;
        if (reflection.flags.hasFlag(models_1.ReflectionFlag.Private) &&
            this.excludePrivate) {
            return true;
        }
        if (reflection.flags.hasFlag(models_1.ReflectionFlag.Protected) &&
            this.excludeProtected) {
            return true;
        }
        if (!comment) {
            // We haven't moved comments from the parent for signatures without a direct
            // comment, so don't exclude those due to not being documented.
            if (reflection.kindOf(models_1.ReflectionKind.CallSignature |
                models_1.ReflectionKind.ConstructorSignature) &&
                reflection.parent?.comment) {
                return false;
            }
            if (this.excludeNotDocumented) {
                // Don't let excludeNotDocumented remove parameters.
                if (!(reflection instanceof models_1.DeclarationReflection) &&
                    !(reflection instanceof models_1.SignatureReflection)) {
                    return false;
                }
                if (!reflection.kindOf(this.excludeNotDocumentedKinds)) {
                    return false;
                }
                // excludeNotDocumented should hide a module only if it has no visible children
                if (reflection.kindOf(models_1.ReflectionKind.SomeModule)) {
                    if (!reflection.children) {
                        return true;
                    }
                    return reflection.children.every((child) => this.isHidden(child));
                }
                // signature containers should only be hidden if all their signatures are hidden
                if (reflection.kindOf(models_1.ReflectionKind.SignatureContainer)) {
                    return reflection
                        .getAllSignatures()
                        .every((child) => this.isHidden(child));
                }
                // excludeNotDocumented should never hide parts of "type" reflections
                return inTypeLiteral(reflection) === false;
            }
            return false;
        }
        const isHidden = comment.hasModifier("@hidden") ||
            comment.hasModifier("@ignore") ||
            (comment.hasModifier("@internal") && this.excludeInternal);
        if (isHidden &&
            reflection.kindOf(models_1.ReflectionKind.ContainsCallSignatures)) {
            return reflection
                .getNonIndexSignatures()
                .every((sig) => {
                return !sig.comment || this.isHidden(sig);
            });
        }
        return isHidden;
    }
};
__decorate([
    (0, utils_1.BindOption)("excludeTags")
], CommentPlugin.prototype, "excludeTags", void 0);
__decorate([
    (0, utils_1.BindOption)("excludeInternal")
], CommentPlugin.prototype, "excludeInternal", void 0);
__decorate([
    (0, utils_1.BindOption)("excludePrivate")
], CommentPlugin.prototype, "excludePrivate", void 0);
__decorate([
    (0, utils_1.BindOption)("excludeProtected")
], CommentPlugin.prototype, "excludeProtected", void 0);
__decorate([
    (0, utils_1.BindOption)("excludeNotDocumented")
], CommentPlugin.prototype, "excludeNotDocumented", void 0);
CommentPlugin = __decorate([
    (0, components_1.Component)({ name: "comment" })
], CommentPlugin);
exports.CommentPlugin = CommentPlugin;
function inTypeLiteral(refl) {
    while (refl) {
        if (refl.kind === models_1.ReflectionKind.TypeLiteral) {
            return true;
        }
        refl = refl.parent;
    }
    return false;
}
// Moves tags like `@param foo.bar docs for bar` into the `bar` property of the `foo` parameter.
function moveNestedParamTags(comment, parameter) {
    const visitor = {
        reflection(target) {
            const tags = comment.blockTags.filter((t) => t.tag === "@param" &&
                t.name?.startsWith(`${parameter.name}.`));
            for (const tag of tags) {
                const path = tag.name.split(".");
                path.shift();
                const child = target.declaration.getChildByName(path);
                if (child && !child.comment) {
                    child.comment = new models_1.Comment(models_1.Comment.cloneDisplayParts(tag.content));
                }
            }
        },
        // #1876, also do this for unions/intersections.
        union(u) {
            u.types.forEach((t) => t.visit(visitor));
        },
        intersection(i) {
            i.types.forEach((t) => t.visit(visitor));
        },
    };
    parameter.type?.visit(visitor);
}
function movePropertyTags(comment, container) {
    const propTags = comment.blockTags.filter((tag) => tag.tag === "@prop" || tag.tag === "@property");
    comment.removeTags("@prop");
    comment.removeTags("@property");
    for (const prop of propTags) {
        if (!prop.name)
            continue;
        const child = container.getChildByName(prop.name);
        if (child) {
            child.comment = new models_1.Comment(models_1.Comment.cloneDisplayParts(prop.content));
        }
    }
}
function mergeSeeTags(comment) {
    const see = comment.getTags("@see");
    if (see.length < 2)
        return;
    const index = comment.blockTags.indexOf(see[0]);
    comment.removeTags("@see");
    see[0].content = see.flatMap((part) => [
        { kind: "text", text: " - " },
        ...part.content,
        { kind: "text", text: "\n" },
    ]);
    comment.blockTags.splice(index, 0, see[0]);
}
