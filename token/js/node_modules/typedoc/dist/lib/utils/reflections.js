"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.discoverAllReferenceTypes = void 0;
const models_1 = require("../models");
function discoverAllReferenceTypes(project, forExportValidation) {
    let current = project;
    const queue = [];
    const result = [];
    const visitor = (0, models_1.makeRecursiveVisitor)({
        reference(type) {
            result.push({ type, owner: current });
        },
        reflection(type) {
            queue.push(type.declaration);
        },
    });
    const add = (item) => {
        if (!item)
            return;
        if (item instanceof models_1.Reflection) {
            queue.push(item);
        }
        else {
            queue.push(...item);
        }
    };
    do {
        if (current instanceof models_1.ContainerReflection) {
            add(current.children);
        }
        if (current instanceof models_1.DeclarationReflection) {
            current.type?.visit(visitor);
            add(current.typeParameters);
            add(current.signatures);
            add(current.indexSignature);
            add(current.getSignature);
            add(current.setSignature);
            current.overwrites?.visit(visitor);
            current.implementedTypes?.forEach((type) => type.visit(visitor));
            if (!forExportValidation) {
                current.inheritedFrom?.visit(visitor);
                current.implementationOf?.visit(visitor);
                current.extendedTypes?.forEach((t) => t.visit(visitor));
                current.extendedBy?.forEach((t) => t.visit(visitor));
                current.implementedBy?.forEach((t) => t.visit(visitor));
            }
        }
        if (current instanceof models_1.SignatureReflection) {
            add(current.parameters);
            add(current.typeParameters);
            current.type?.visit(visitor);
            current.overwrites?.visit(visitor);
            if (!forExportValidation) {
                current.inheritedFrom?.visit(visitor);
                current.implementationOf?.visit(visitor);
            }
        }
        if (current instanceof models_1.ParameterReflection) {
            current.type?.visit(visitor);
        }
        if (current instanceof models_1.TypeParameterReflection) {
            current.type?.visit(visitor);
            current.default?.visit(visitor);
        }
    } while ((current = queue.shift()));
    return result;
}
exports.discoverAllReferenceTypes = discoverAllReferenceTypes;
