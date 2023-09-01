"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.resolveDeclarationReference = void 0;
const assert_1 = require("assert");
const models_1 = require("../../models");
const utils_1 = require("../../utils");
function resolveReferenceReflection(ref) {
    if (ref instanceof models_1.ReferenceReflection) {
        return ref.getTargetReflectionDeep();
    }
    return ref;
}
function resolveDeclarationReference(reflection, ref) {
    let high = [];
    let low = [];
    if (ref.moduleSource) {
        high =
            reflection.project.children?.filter((c) => c.kindOf(models_1.ReflectionKind.SomeModule) &&
                c.name === ref.moduleSource) || [];
    }
    else if (ref.resolutionStart === "global") {
        high.push(reflection.project);
    }
    else {
        (0, assert_1.ok)(ref.resolutionStart === "local");
        // TypeScript's behavior is to first try to resolve links via variable scope, and then
        // if the link still hasn't been found, check either siblings (if comment belongs to a member)
        // or otherwise children.
        let refl = reflection;
        if (refl.kindOf(models_1.ReflectionKind.ExportContainer)) {
            high.push(refl);
        }
        while (refl.parent) {
            refl = refl.parent;
            if (refl.kindOf(models_1.ReflectionKind.ExportContainer)) {
                high.push(refl);
            }
            else {
                low.push(refl);
            }
        }
        if (reflection.kindOf(models_1.ReflectionKind.SomeMember)) {
            high.push(reflection.parent);
        }
        else if (reflection.kindOf(models_1.ReflectionKind.SomeSignature) &&
            reflection.parent.kindOf(models_1.ReflectionKind.SomeMember)) {
            high.push(reflection.parent.parent);
        }
        else if (high[0] !== reflection) {
            if (reflection.parent instanceof models_1.ContainerReflection) {
                high.push(...(reflection.parent.children?.filter((c) => c.name === reflection.name) || []));
            }
            else {
                high.push(reflection);
            }
        }
    }
    if (ref.symbolReference) {
        for (const part of ref.symbolReference.path || []) {
            const high2 = high;
            high = [];
            const low2 = low;
            low = [];
            for (const refl of high2) {
                const resolved = resolveSymbolReferencePart(refl, part);
                high.push(...resolved.high.map(resolveReferenceReflection));
                low.push(...resolved.low.map(resolveReferenceReflection));
            }
            for (const refl of low2) {
                const resolved = resolveSymbolReferencePart(refl, part);
                low.push(...resolved.high.map(resolveReferenceReflection));
                low.push(...resolved.low.map(resolveReferenceReflection));
            }
        }
        if (ref.symbolReference.meaning) {
            high = filterMapByMeaning(high, ref.symbolReference.meaning);
            low = filterMapByMeaning(low, ref.symbolReference.meaning);
        }
    }
    return high[0] || low[0];
}
exports.resolveDeclarationReference = resolveDeclarationReference;
function filterMapByMeaning(reflections, meaning) {
    return (0, utils_1.filterMap)(reflections, (refl) => {
        const kwResolved = resolveKeyword(refl, meaning.keyword) || [];
        if (meaning.label) {
            return kwResolved.find((r) => r.comment?.label === meaning.label);
        }
        return kwResolved[meaning.index || 0];
    });
}
function resolveKeyword(refl, kw) {
    switch (kw) {
        case undefined:
            return refl instanceof models_1.DeclarationReflection && refl.signatures
                ? refl.signatures
                : [refl];
        case "class":
            if (refl.kindOf(models_1.ReflectionKind.Class))
                return [refl];
            break;
        case "interface":
            if (refl.kindOf(models_1.ReflectionKind.Interface))
                return [refl];
            break;
        case "type":
            if (refl.kindOf(models_1.ReflectionKind.SomeType))
                return [refl];
            break;
        case "enum":
            if (refl.kindOf(models_1.ReflectionKind.Enum))
                return [refl];
            break;
        case "namespace":
            if (refl.kindOf(models_1.ReflectionKind.SomeModule))
                return [refl];
            break;
        case "function":
            if (refl.kindOf(models_1.ReflectionKind.FunctionOrMethod)) {
                return refl.signatures;
            }
            break;
        case "var":
            if (refl.kindOf(models_1.ReflectionKind.Variable))
                return [refl];
            break;
        case "new":
        case "constructor":
            if (refl.kindOf(models_1.ReflectionKind.ClassOrInterface | models_1.ReflectionKind.TypeLiteral)) {
                const ctor = refl.children?.find((c) => c.kindOf(models_1.ReflectionKind.Constructor));
                return ctor?.signatures;
            }
            break;
        case "member":
            if (refl.kindOf(models_1.ReflectionKind.SomeMember))
                return [refl];
            break;
        case "event":
            // Never resolve. Nobody should use this.
            // It's required by the grammar, but is not documented by TypeDoc
            // nor by the comments in the grammar.
            break;
        case "call":
            return refl.signatures;
        case "index":
            if (refl.indexSignature) {
                return [refl.indexSignature];
            }
            break;
        case "complex":
            if (refl.kindOf(models_1.ReflectionKind.SomeType))
                return [refl];
            break;
        case "getter":
            if (refl.getSignature) {
                return [refl.getSignature];
            }
            break;
        case "setter":
            if (refl.setSignature) {
                return [refl.setSignature];
            }
            break;
        default:
            (0, utils_1.assertNever)(kw);
    }
}
function resolveSymbolReferencePart(refl, path) {
    let high = [];
    let low = [];
    if (!(refl instanceof models_1.ContainerReflection) || !refl.children) {
        return { high, low };
    }
    switch (path.navigation) {
        // Grammar says resolve via "exports"... as always, reality is more complicated.
        // Check exports first, but also allow this as a general purpose "some child" operator
        // so that resolution doesn't behave very poorly with projects using JSDoc style resolution.
        // Also is more consistent with how TypeScript resolves link tags.
        case ".":
            high = refl.children.filter((r) => r.name === path.path &&
                (r.kindOf(models_1.ReflectionKind.SomeExport) || r.flags.isStatic));
            low = refl.children.filter((r) => r.name === path.path &&
                (!r.kindOf(models_1.ReflectionKind.SomeExport) || !r.flags.isStatic));
            break;
        // Resolve via "members", interface children, class instance properties/accessors/methods,
        // enum members, type literal properties
        case "#":
            high = refl.children.filter((r) => {
                return (r.name === path.path &&
                    r.kindOf(models_1.ReflectionKind.SomeMember) &&
                    !r.flags.isStatic);
            });
            break;
        // Resolve via "locals"... treat this as a stricter `.` which only supports traversing
        // module/namespace exports since TypeDoc doesn't support documenting locals.
        case "~":
            if (refl.kindOf(models_1.ReflectionKind.SomeModule | models_1.ReflectionKind.Project)) {
                high = refl.children.filter((r) => r.name === path.path);
            }
            break;
    }
    return { high, low };
}
