"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.type = exports.validateStateIsClean = void 0;
const models_1 = require("../../../../models");
const utils_1 = require("../../../../utils");
const lib_1 = require("../../lib");
const assert_1 = require("assert");
const EXPORTABLE = models_1.ReflectionKind.Class |
    models_1.ReflectionKind.Interface |
    models_1.ReflectionKind.Enum |
    models_1.ReflectionKind.TypeAlias |
    models_1.ReflectionKind.Function |
    models_1.ReflectionKind.Variable;
const nameCollisionCache = new WeakMap();
function getNameCollisionCount(project, name) {
    let collisions = nameCollisionCache.get(project);
    if (collisions === undefined) {
        collisions = {};
        for (const reflection of project.getReflectionsByKind(EXPORTABLE)) {
            collisions[reflection.name] = (collisions[reflection.name] ?? 0) + 1;
        }
        nameCollisionCache.set(project, collisions);
    }
    return collisions[name] ?? 0;
}
/**
 * Returns a (hopefully) globally unique path for the given reflection.
 *
 * This only works for exportable symbols, so e.g. methods are not affected by this.
 *
 * If the given reflection has a globally unique name already, then it will be returned as is. If the name is
 * ambiguous (i.e. there are two classes with the same name in different namespaces), then the namespaces path of the
 * reflection will be returned.
 */
function getUniquePath(reflection) {
    if (reflection.kindOf(EXPORTABLE)) {
        if (getNameCollisionCount(reflection.project, reflection.name) >= 2) {
            return getNamespacedPath(reflection);
        }
    }
    return [reflection];
}
function getNamespacedPath(reflection) {
    const path = [reflection];
    let parent = reflection.parent;
    while (parent?.kindOf(models_1.ReflectionKind.Namespace)) {
        path.unshift(parent);
        parent = parent.parent;
    }
    return path;
}
function renderUniquePath(context, reflection) {
    return (0, lib_1.join)(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "."), getUniquePath(reflection), (item) => (utils_1.JSX.createElement("a", { href: context.urlTo(item), class: "tsd-signature-type " + (0, lib_1.getKindClass)(item) }, item.name)));
}
let indentationDepth = 0;
function includeIndentation() {
    return indentationDepth > 0 ? utils_1.JSX.createElement("span", null, "\u00A0".repeat(indentationDepth * 4)) : utils_1.JSX.createElement(utils_1.JSX.Fragment, null);
}
function validateStateIsClean(page) {
    (0, assert_1.ok)(indentationDepth === 0, `Rendering ${page}: Indentation depth increment/decrement not matched: ${indentationDepth}`);
}
exports.validateStateIsClean = validateStateIsClean;
// The type helper accepts an optional needsParens parameter that is checked
// if an inner type may result in invalid output without them. For example:
// 1 | 2[] !== (1 | 2)[]
// () => 1 | 2 !== (() => 1) | 2
const typeRenderers = {
    array(context, type) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            renderType(context, type.elementType, models_1.TypeContext.arrayElement),
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "[]")));
    },
    conditional(context, type) {
        indentationDepth++;
        const parts = [
            renderType(context, type.checkType, models_1.TypeContext.conditionalCheck),
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, " extends "),
            renderType(context, type.extendsType, models_1.TypeContext.conditionalExtends),
            utils_1.JSX.createElement("br", null),
            includeIndentation(),
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "? "),
            renderType(context, type.trueType, models_1.TypeContext.conditionalTrue),
            utils_1.JSX.createElement("br", null),
            includeIndentation(),
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ": "),
            renderType(context, type.falseType, models_1.TypeContext.conditionalFalse),
        ];
        indentationDepth--;
        return utils_1.JSX.createElement(utils_1.JSX.Fragment, null, parts);
    },
    indexedAccess(context, type) {
        let indexType = renderType(context, type.indexType, models_1.TypeContext.indexedIndex);
        if (type.objectType instanceof models_1.ReferenceType &&
            type.objectType.reflection &&
            type.indexType instanceof models_1.LiteralType &&
            typeof type.indexType.value === "string") {
            const childReflection = type.objectType.reflection.getChildByName([type.indexType.value]);
            if (childReflection) {
                indexType = utils_1.JSX.createElement("a", { href: context.urlTo(childReflection) }, indexType);
            }
        }
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            renderType(context, type.objectType, models_1.TypeContext.indexedObject),
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "["),
            indexType,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "]")));
    },
    inferred(context, type) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "infer "),
            " ",
            utils_1.JSX.createElement("span", { class: "tsd-kind-type-parameter" }, type.name),
            type.constraint && (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, " extends "),
                renderType(context, type.constraint, models_1.TypeContext.inferredConstraint)))));
    },
    intersection(context, type) {
        return (0, lib_1.join)(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, " & "), type.types, (item) => renderType(context, item, models_1.TypeContext.intersectionElement));
    },
    intrinsic(_context, type) {
        return utils_1.JSX.createElement("span", { class: "tsd-signature-type" }, type.name);
    },
    literal(_context, type) {
        return utils_1.JSX.createElement("span", { class: "tsd-signature-type" }, (0, lib_1.stringify)(type.value));
    },
    mapped(context, type) {
        indentationDepth++;
        const parts = [utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "{"), utils_1.JSX.createElement("br", null), includeIndentation()];
        switch (type.readonlyModifier) {
            case "+":
                parts.push(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "readonly "));
                break;
            case "-":
                parts.push(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "-readonly "));
                break;
        }
        parts.push(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "["), utils_1.JSX.createElement("span", { class: "tsd-kind-type-parameter" }, type.parameter), utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, " in "), renderType(context, type.parameterType, models_1.TypeContext.mappedParameter));
        if (type.nameType) {
            parts.push(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, " as "), renderType(context, type.nameType, models_1.TypeContext.mappedName));
        }
        parts.push(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "]"));
        switch (type.optionalModifier) {
            case "+":
                parts.push(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "?: "));
                break;
            case "-":
                parts.push(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "-?: "));
                break;
            default:
                parts.push(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ": "));
        }
        parts.push(renderType(context, type.templateType, models_1.TypeContext.mappedTemplate));
        indentationDepth--;
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            parts,
            utils_1.JSX.createElement("br", null),
            includeIndentation(),
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "}")));
    },
    namedTupleMember(context, type) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            type.name,
            type.isOptional ? (utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "?: ")) : (utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ": ")),
            renderType(context, type.element, models_1.TypeContext.tupleElement)));
    },
    optional(context, type) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            renderType(context, type.elementType, models_1.TypeContext.optionalElement),
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "?")));
    },
    predicate(context, type) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            !!type.asserts && utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "asserts "),
            utils_1.JSX.createElement("span", { class: "tsd-kind-parameter" }, type.name),
            !!type.targetType && (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, " is "),
                renderType(context, type.targetType, models_1.TypeContext.predicateTarget)))));
    },
    query(context, type) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "typeof "),
            renderType(context, type.queryType, models_1.TypeContext.queryTypeTarget)));
    },
    reference(context, type) {
        const reflection = type.reflection;
        let name;
        if (reflection) {
            if (reflection.kindOf(models_1.ReflectionKind.TypeParameter)) {
                // Don't generate a link as it will always point to this page.
                name = utils_1.JSX.createElement("span", { class: "tsd-signature-type tsd-kind-type-parameter" }, reflection.name);
            }
            else {
                name = renderUniquePath(context, reflection);
            }
        }
        else if (type.externalUrl) {
            name = (utils_1.JSX.createElement("a", { href: type.externalUrl, class: "tsd-signature-type external", target: "_blank" }, type.name));
        }
        else if (type.refersToTypeParameter) {
            name = utils_1.JSX.createElement("span", { class: "tsd-signature-type tsd-kind-type-parameter" }, type.name);
        }
        else {
            name = utils_1.JSX.createElement("span", { class: "tsd-signature-type " }, type.name);
        }
        if (type.typeArguments?.length) {
            return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                name,
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "<"),
                (0, lib_1.join)(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ", "), type.typeArguments, (item) => renderType(context, item, models_1.TypeContext.referenceTypeArgument)),
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ">")));
        }
        return name;
    },
    reflection(context, type) {
        const members = [];
        const children = type.declaration.children || [];
        indentationDepth++;
        for (const item of children) {
            if (item.getSignature && item.setSignature) {
                members.push(utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                    utils_1.JSX.createElement("span", { class: (0, lib_1.getKindClass)(item) }, item.name),
                    utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ": "),
                    renderType(context, item.getSignature.type, models_1.TypeContext.none)));
                continue;
            }
            if (item.getSignature) {
                members.push(utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                    utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "get "),
                    utils_1.JSX.createElement("span", { class: (0, lib_1.getKindClass)(item.getSignature) }, item.name),
                    utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "(): "),
                    renderType(context, item.getSignature.type, models_1.TypeContext.none)));
                continue;
            }
            if (item.setSignature) {
                members.push(utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                    utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "set "),
                    utils_1.JSX.createElement("span", { class: (0, lib_1.getKindClass)(item.setSignature) }, item.name),
                    utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "("),
                    item.setSignature.parameters?.map((item) => (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                        item.name,
                        utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ": "),
                        renderType(context, item.type, models_1.TypeContext.none)))),
                    utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ")")));
                continue;
            }
            if (item.signatures) {
                for (const sig of item.signatures) {
                    members.push(utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                        utils_1.JSX.createElement("span", { class: (0, lib_1.getKindClass)(sig) }, item.name),
                        item.flags.isOptional && utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "?"),
                        context.memberSignatureTitle(sig, {
                            hideName: true,
                            arrowStyle: true,
                        })));
                }
                continue;
            }
            members.push(utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                utils_1.JSX.createElement("span", { class: (0, lib_1.getKindClass)(item) }, item.name),
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, item.flags.isOptional ? "?: " : ": "),
                renderType(context, item.type, models_1.TypeContext.none)));
        }
        if (type.declaration.indexSignature) {
            const index = type.declaration.indexSignature;
            members.push(utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                "[",
                utils_1.JSX.createElement("span", { class: (0, lib_1.getKindClass)(type.declaration.indexSignature) }, index.parameters[0].name),
                ":",
                " ",
                renderType(context, index.parameters[0].type, models_1.TypeContext.none),
                "]",
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ": "),
                renderType(context, index.type, models_1.TypeContext.none)));
        }
        if (!members.length && type.declaration.signatures?.length === 1) {
            indentationDepth--;
            return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "("),
                context.memberSignatureTitle(type.declaration.signatures[0], {
                    hideName: true,
                    arrowStyle: true,
                }),
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ")")));
        }
        for (const item of type.declaration.signatures || []) {
            members.push(context.memberSignatureTitle(item, { hideName: true }));
        }
        if (members.length) {
            const membersWithSeparators = members.flatMap((m) => [
                includeIndentation(),
                m,
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "; "),
                utils_1.JSX.createElement("br", null),
            ]);
            membersWithSeparators.pop();
            indentationDepth--;
            return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" },
                    "{",
                    " "),
                utils_1.JSX.createElement("br", null),
                membersWithSeparators,
                utils_1.JSX.createElement("br", null),
                includeIndentation(),
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "}")));
        }
        indentationDepth--;
        return utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "{}");
    },
    rest(context, type) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "..."),
            renderType(context, type.elementType, models_1.TypeContext.restElement)));
    },
    templateLiteral(context, type) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "`"),
            type.head && utils_1.JSX.createElement("span", { class: "tsd-signature-type" }, type.head),
            type.tail.map((item) => (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "${"),
                renderType(context, item[0], models_1.TypeContext.templateLiteralElement),
                utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "}"),
                item[1] && utils_1.JSX.createElement("span", { class: "tsd-signature-type" }, item[1])))),
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "`")));
    },
    tuple(context, type) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "["),
            (0, lib_1.join)(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ", "), type.elements, (item) => renderType(context, item, models_1.TypeContext.tupleElement)),
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "]")));
    },
    typeOperator(context, type) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" },
                type.operator,
                " "),
            renderType(context, type.target, models_1.TypeContext.typeOperatorTarget)));
    },
    union(context, type) {
        return (0, lib_1.join)(utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, " | "), type.types, (item) => renderType(context, item, models_1.TypeContext.unionElement));
    },
    unknown(_context, type) {
        return utils_1.JSX.createElement(utils_1.JSX.Fragment, null, type.name);
    },
};
function renderType(context, type, where) {
    if (!type) {
        return utils_1.JSX.createElement("span", { class: "tsd-signature-type" }, "any");
    }
    const renderFn = typeRenderers[type.type];
    const rendered = renderFn(context, type);
    if (type.needsParenthesis(where)) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, "("),
            rendered,
            utils_1.JSX.createElement("span", { class: "tsd-signature-symbol" }, ")")));
    }
    return rendered;
}
function type(context, type) {
    return renderType(context, type, models_1.TypeContext.none);
}
exports.type = type;
