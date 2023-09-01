"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.ImplementsPlugin = void 0;
const typescript_1 = __importDefault(require("typescript"));
const application_events_1 = require("../../application-events");
const index_1 = require("../../models/reflections/index");
const types_1 = require("../../models/types");
const array_1 = require("../../utils/array");
const components_1 = require("../components");
const converter_1 = require("../converter");
const utils_1 = require("../../utils");
/**
 * A plugin that detects interface implementations of functions and
 * properties on classes and links them.
 */
let ImplementsPlugin = class ImplementsPlugin extends components_1.ConverterComponent {
    constructor() {
        super(...arguments);
        this.resolved = new WeakSet();
        this.postponed = new WeakMap();
    }
    /**
     * Create a new ImplementsPlugin instance.
     */
    initialize() {
        this.listenTo(this.owner, converter_1.Converter.EVENT_RESOLVE_END, this.onResolveEnd);
        this.listenTo(this.owner, converter_1.Converter.EVENT_CREATE_DECLARATION, this.onDeclaration, -1000);
        this.listenTo(this.owner, converter_1.Converter.EVENT_CREATE_SIGNATURE, this.onSignature, 1000);
        this.listenTo(this.application, application_events_1.ApplicationEvents.REVIVE, this.resolve);
    }
    /**
     * Mark all members of the given class to be the implementation of the matching interface member.
     */
    analyzeImplements(project, classReflection, interfaceReflection) {
        handleInheritedComments(classReflection, interfaceReflection);
        if (!interfaceReflection.children) {
            return;
        }
        interfaceReflection.children.forEach((interfaceMember) => {
            const classMember = findMatchingMember(interfaceMember, classReflection);
            if (!classMember) {
                return;
            }
            const interfaceMemberName = interfaceReflection.name + "." + interfaceMember.name;
            classMember.implementationOf =
                types_1.ReferenceType.createResolvedReference(interfaceMemberName, interfaceMember, project);
            const intSigs = interfaceMember.signatures ||
                interfaceMember.type?.visit({
                    reflection: (r) => r.declaration.signatures,
                });
            const clsSigs = classMember.signatures ||
                classMember.type?.visit({
                    reflection: (r) => r.declaration.signatures,
                });
            if (intSigs && clsSigs) {
                for (const [clsSig, intSig] of (0, array_1.zip)(clsSigs, intSigs)) {
                    if (clsSig.implementationOf) {
                        const target = intSig.parent.kindOf(index_1.ReflectionKind.FunctionOrMethod)
                            ? intSig
                            : intSig.parent.parent;
                        clsSig.implementationOf =
                            types_1.ReferenceType.createResolvedReference(clsSig.implementationOf.name, target, project);
                    }
                }
            }
            handleInheritedComments(classMember, interfaceMember);
        });
    }
    analyzeInheritance(project, reflection) {
        const extendedTypes = (0, array_1.filterMap)(reflection.extendedTypes ?? [], (type) => {
            return type instanceof types_1.ReferenceType &&
                type.reflection instanceof index_1.DeclarationReflection
                ? type
                : void 0;
        });
        for (const parent of extendedTypes) {
            handleInheritedComments(reflection, parent.reflection);
            for (const parentMember of parent.reflection.children ?? []) {
                const child = findMatchingMember(parentMember, reflection);
                if (child) {
                    const key = child.overwrites
                        ? "overwrites"
                        : "inheritedFrom";
                    for (const [childSig, parentSig] of (0, array_1.zip)(child.signatures ?? [], parentMember.signatures ?? [])) {
                        childSig[key] = types_1.ReferenceType.createResolvedReference(`${parent.name}.${parentMember.name}`, parentSig, project);
                    }
                    child[key] = types_1.ReferenceType.createResolvedReference(`${parent.name}.${parentMember.name}`, parentMember, project);
                    handleInheritedComments(child, parentMember);
                }
            }
        }
    }
    onResolveEnd(context) {
        this.resolve(context.project);
    }
    resolve(project) {
        for (const reflection of Object.values(project.reflections)) {
            if (reflection instanceof index_1.DeclarationReflection) {
                this.tryResolve(project, reflection);
            }
        }
    }
    tryResolve(project, reflection) {
        const requirements = (0, array_1.filterMap)([
            ...(reflection.implementedTypes ?? []),
            ...(reflection.extendedTypes ?? []),
        ], (type) => {
            return type instanceof types_1.ReferenceType ? type.reflection : void 0;
        });
        if (requirements.every((req) => this.resolved.has(req))) {
            this.doResolve(project, reflection);
            this.resolved.add(reflection);
            for (const refl of this.postponed.get(reflection) ?? []) {
                this.tryResolve(project, refl);
            }
            this.postponed.delete(reflection);
        }
        else {
            for (const req of requirements) {
                const future = this.postponed.get(req) ?? new Set();
                future.add(reflection);
                this.postponed.set(req, future);
            }
        }
    }
    doResolve(project, reflection) {
        if (reflection.kindOf(index_1.ReflectionKind.Class) &&
            reflection.implementedTypes) {
            reflection.implementedTypes.forEach((type) => {
                if (!(type instanceof types_1.ReferenceType)) {
                    return;
                }
                if (type.reflection &&
                    type.reflection.kindOf(index_1.ReflectionKind.ClassOrInterface)) {
                    this.analyzeImplements(project, reflection, type.reflection);
                }
            });
        }
        if (reflection.kindOf(index_1.ReflectionKind.ClassOrInterface) &&
            reflection.extendedTypes) {
            this.analyzeInheritance(project, reflection);
        }
    }
    getExtensionInfo(context, reflection) {
        if (!reflection || !reflection.kindOf(index_1.ReflectionKind.Inheritable)) {
            return;
        }
        // Need this because we re-use reflections for type literals.
        if (!reflection.parent?.kindOf(index_1.ReflectionKind.ClassOrInterface)) {
            return;
        }
        const symbol = context.project.getSymbolFromReflection(reflection.parent);
        if (!symbol) {
            return;
        }
        const declaration = symbol
            .getDeclarations()
            ?.find((n) => typescript_1.default.isClassDeclaration(n) || typescript_1.default.isInterfaceDeclaration(n));
        if (!declaration) {
            return;
        }
        return { symbol, declaration };
    }
    onSignature(context, reflection) {
        this.onDeclaration(context, reflection.parent);
    }
    /**
     * Responsible for setting the {@link DeclarationReflection.inheritedFrom},
     * {@link DeclarationReflection.overwrites}, and {@link DeclarationReflection.implementationOf}
     * properties on the provided reflection temporarily, these links will be replaced
     * during the resolve step with links which actually point to the right place.
     */
    onDeclaration(context, reflection) {
        const info = this.getExtensionInfo(context, reflection);
        if (!info) {
            return;
        }
        if (reflection.kind === index_1.ReflectionKind.Constructor) {
            const ctor = info.declaration.members.find(typescript_1.default.isConstructorDeclaration);
            constructorInheritance(context, reflection, info.declaration, ctor);
            return;
        }
        const childType = reflection.flags.isStatic
            ? context.checker.getTypeOfSymbolAtLocation(info.symbol, info.declaration)
            : context.checker.getDeclaredTypeOfSymbol(info.symbol);
        const property = findProperty(reflection, childType);
        if (!property) {
            // We're probably broken... but I don't think this should be fatal.
            context.logger.warn(`Failed to retrieve${reflection.flags.isStatic ? " static" : ""} member "${reflection.escapedName ?? reflection.name}" of "${reflection.parent?.name}" for inheritance analysis. Please report a bug.`);
            return;
        }
        // Need to check both extends and implements clauses.
        out: for (const clause of info.declaration.heritageClauses ?? []) {
            // No point checking implemented types for static members, they won't exist.
            if (reflection.flags.isStatic &&
                clause.token === typescript_1.default.SyntaxKind.ImplementsKeyword) {
                continue;
            }
            for (const expr of clause.types) {
                const parentType = context.checker.getTypeAtLocation(reflection.flags.isStatic ? expr.expression : expr);
                const parentProperty = findProperty(reflection, parentType);
                if (parentProperty) {
                    const isInherit = property
                        .getDeclarations()
                        ?.some((d) => d.parent !== info.declaration) ??
                        true;
                    createLink(context, reflection, clause, expr, parentProperty, isInherit);
                    // Can't always break because we need to also set `implementationOf` if we
                    // inherit from a base class and also implement an interface.
                    if (clause.token === typescript_1.default.SyntaxKind.ImplementsKeyword) {
                        break out;
                    }
                }
            }
        }
    }
};
ImplementsPlugin = __decorate([
    (0, components_1.Component)({ name: "implements" })
], ImplementsPlugin);
exports.ImplementsPlugin = ImplementsPlugin;
function constructorInheritance(context, reflection, childDecl, constructorDecl) {
    const extendsClause = childDecl.heritageClauses?.find((cl) => cl.token === typescript_1.default.SyntaxKind.ExtendsKeyword);
    if (!extendsClause)
        return;
    const name = `${extendsClause.types[0].getText()}.constructor`;
    const key = constructorDecl ? "overwrites" : "inheritedFrom";
    reflection[key] ?? (reflection[key] = types_1.ReferenceType.createBrokenReference(name, context.project));
    for (const sig of reflection.signatures ?? []) {
        sig[key] ?? (sig[key] = types_1.ReferenceType.createBrokenReference(name, context.project));
    }
}
function findProperty(reflection, parent) {
    return parent.getProperties().find((prop) => {
        return reflection.escapedName
            ? prop.escapedName === reflection.escapedName
            : prop.name === reflection.name;
    });
}
function createLink(context, reflection, clause, expr, symbol, isOverwrite) {
    const project = context.project;
    const name = `${expr.expression.getText()}.${(0, utils_1.getHumanName)(symbol.name)}`;
    link(reflection);
    link(reflection.getSignature);
    link(reflection.setSignature);
    link(reflection.indexSignature);
    for (const sig of reflection.signatures ?? []) {
        link(sig);
    }
    // Intentionally create broken links here. These will be replaced with real links during
    // resolution if we can do so.
    function link(target) {
        if (!target)
            return;
        if (clause.token === typescript_1.default.SyntaxKind.ImplementsKeyword) {
            target.implementationOf ?? (target.implementationOf = types_1.ReferenceType.createBrokenReference(name, project));
            return;
        }
        if (isOverwrite) {
            target.inheritedFrom ?? (target.inheritedFrom = types_1.ReferenceType.createBrokenReference(name, project));
        }
        else {
            target.overwrites ?? (target.overwrites = types_1.ReferenceType.createBrokenReference(name, project));
        }
    }
}
/**
 * Responsible for copying comments from "parent" reflections defined
 * in either a base class or implemented interface to the child class.
 */
function handleInheritedComments(child, parent) {
    copyComment(child, parent);
    if (parent.kindOf(index_1.ReflectionKind.Property) &&
        child.kindOf(index_1.ReflectionKind.Accessor)) {
        if (child.getSignature) {
            copyComment(child.getSignature, parent);
            child.getSignature.implementationOf = child.implementationOf;
        }
        if (child.setSignature) {
            copyComment(child.setSignature, parent);
            child.setSignature.implementationOf = child.implementationOf;
        }
    }
    if (parent.kindOf(index_1.ReflectionKind.Accessor) &&
        child.kindOf(index_1.ReflectionKind.Accessor)) {
        if (parent.getSignature && child.getSignature) {
            copyComment(child.getSignature, parent.getSignature);
        }
        if (parent.setSignature && child.setSignature) {
            copyComment(child.setSignature, parent.setSignature);
        }
    }
    if (parent.kindOf(index_1.ReflectionKind.FunctionOrMethod) &&
        parent.signatures &&
        child.signatures) {
        for (const [cs, ps] of (0, array_1.zip)(child.signatures, parent.signatures)) {
            copyComment(cs, ps);
        }
    }
    else if (parent.kindOf(index_1.ReflectionKind.Property) &&
        parent.type instanceof types_1.ReflectionType &&
        parent.type.declaration.signatures &&
        child.signatures) {
        for (const [cs, ps] of (0, array_1.zip)(child.signatures, parent.type.declaration.signatures)) {
            copyComment(cs, ps);
        }
    }
}
/**
 * Copy the comment of the source reflection to the target reflection with a JSDoc style copy
 * function. The TSDoc copy function is in the InheritDocPlugin.
 */
function copyComment(target, source) {
    if (target.comment) {
        // We might still want to copy, if the child has a JSDoc style inheritDoc tag.
        const tag = target.comment.getTag("@inheritDoc");
        if (!tag || tag.name) {
            return;
        }
    }
    if (!source.comment) {
        return;
    }
    target.comment = source.comment.clone();
    if (target instanceof index_1.DeclarationReflection &&
        source instanceof index_1.DeclarationReflection) {
        for (const [tt, ts] of (0, array_1.zip)(target.typeParameters || [], source.typeParameters || [])) {
            copyComment(tt, ts);
        }
    }
    if (target instanceof index_1.SignatureReflection &&
        source instanceof index_1.SignatureReflection) {
        for (const [tt, ts] of (0, array_1.zip)(target.typeParameters || [], source.typeParameters || [])) {
            copyComment(tt, ts);
        }
        for (const [pt, ps] of (0, array_1.zip)(target.parameters || [], source.parameters || [])) {
            copyComment(pt, ps);
        }
    }
}
function findMatchingMember(toMatch, container) {
    return container.children?.find((child) => child.name == toMatch.name &&
        child.flags.isStatic === toMatch.flags.isStatic);
}
