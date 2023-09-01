"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Deserializer = void 0;
const assert_1 = require("assert");
const index_1 = require("../models/index");
const array_1 = require("../utils/array");
class Deserializer {
    constructor(app) {
        this.app = app;
        this.deferred = [];
        this.deserializers = [];
        this.activeReflection = [];
        this.reflectionBuilders = {
            declaration(parent, obj) {
                return new index_1.DeclarationReflection(obj.name, obj.kind, parent);
            },
            param(parent, obj) {
                return new index_1.ParameterReflection(obj.name, obj.kind, parent);
            },
            project() {
                throw new Error("Not supported, use Deserializer.reviveProject(s) instead.");
            },
            reference(parent, obj) {
                // Ugly, but we don't have a reference yet!
                return new index_1.ReferenceReflection(obj.name, 
                /* target */ parent, parent);
            },
            signature(parent, obj) {
                return new index_1.SignatureReflection(obj.name, obj.kind, parent);
            },
            typeParam(parent, obj) {
                return new index_1.TypeParameterReflection(obj.name, parent, void 0);
            },
        };
        this.typeBuilders = {
            array(obj, de) {
                return new index_1.ArrayType(de.reviveType(obj.elementType));
            },
            conditional(obj, de) {
                return new index_1.ConditionalType(de.reviveType(obj.checkType), de.reviveType(obj.extendsType), de.reviveType(obj.trueType), de.reviveType(obj.falseType));
            },
            indexedAccess(obj, de) {
                return new index_1.IndexedAccessType(de.reviveType(obj.objectType), de.reviveType(obj.indexType));
            },
            inferred(obj, de) {
                return new index_1.InferredType(obj.name, de.reviveType(obj.constraint));
            },
            intersection(obj, de) {
                return new index_1.IntersectionType(obj.types.map((t) => de.reviveType(t)));
            },
            intrinsic(obj) {
                return new index_1.IntrinsicType(obj.name);
            },
            literal(obj) {
                if (obj.value && typeof obj.value === "object") {
                    return new index_1.LiteralType(BigInt(`${obj.value.negative ? "-" : ""}${obj.value.value}`));
                }
                return new index_1.LiteralType(obj.value);
            },
            mapped(obj, de) {
                return new index_1.MappedType(obj.parameter, de.reviveType(obj.parameterType), de.reviveType(obj.templateType), obj.readonlyModifier, obj.optionalModifier, de.reviveType(obj.nameType));
            },
            optional(obj, de) {
                return new index_1.OptionalType(de.reviveType(obj.elementType));
            },
            predicate(obj, de) {
                return new index_1.PredicateType(obj.name, obj.asserts, de.reviveType(obj.targetType));
            },
            query(obj, de) {
                return new index_1.QueryType(de.reviveType(obj.queryType));
            },
            reference(obj) {
                // Correct reference will be restored in fromObject
                return index_1.ReferenceType.createResolvedReference(obj.name, -2, null);
            },
            reflection(obj, de) {
                return new index_1.ReflectionType(de.revive(obj.declaration, (o) => de.constructReflection(o)));
            },
            rest(obj, de) {
                return new index_1.RestType(de.reviveType(obj.elementType));
            },
            templateLiteral(obj, de) {
                return new index_1.TemplateLiteralType(obj.head, obj.tail.map(([t, s]) => [de.reviveType(t), s]));
            },
            tuple(obj, de) {
                return new index_1.TupleType(obj.elements?.map((t) => de.reviveType(t)) || []);
            },
            namedTupleMember(obj, de) {
                return new index_1.NamedTupleMember(obj.name, obj.isOptional, de.reviveType(obj.element));
            },
            typeOperator(obj, de) {
                return new index_1.TypeOperatorType(de.reviveType(obj.target), obj.operator);
            },
            union(obj, de) {
                return new index_1.UnionType(obj.types.map((t) => de.reviveType(t)));
            },
            unknown(obj) {
                return new index_1.UnknownType(obj.name);
            },
        };
        this.oldIdToNewId = {};
    }
    get logger() {
        return this.app.logger;
    }
    addDeserializer(de) {
        (0, array_1.insertPrioritySorted)(this.deserializers, de);
    }
    /**
     * Revive a single project into the structure it was originally created with.
     * This is generally not appropriate for merging multiple projects since projects may
     * contain reflections in their root, not inside a module.
     */
    reviveProject(projectObj, name) {
        (0, assert_1.ok)(this.deferred.length === 0, "Deserializer.defer was called when not deserializing");
        const project = new index_1.ProjectReflection(name || projectObj.name);
        project.registerReflection(project);
        this.project = project;
        this.oldIdToNewId = { [projectObj.id]: project.id };
        this.fromObject(project, projectObj);
        const deferred = this.deferred;
        this.deferred = [];
        for (const def of deferred) {
            def(project);
        }
        (0, assert_1.ok)(this.deferred.length === 0, "Work may not be double deferred when deserializing.");
        (0, assert_1.ok)(this.activeReflection.length === 0, "Imbalanced reflection deserialization");
        this.project = undefined;
        this.oldIdToNewId = {};
        return project;
    }
    reviveProjects(name, projects) {
        if (projects.length === 1) {
            return this.reviveProject(projects[0], name);
        }
        const project = new index_1.ProjectReflection(name);
        project.children = [];
        this.project = project;
        for (const proj of projects) {
            (0, assert_1.ok)(this.deferred.length === 0, "Deserializer.defer was called when not deserializing");
            const projModule = new index_1.DeclarationReflection(proj.name, index_1.ReflectionKind.Module, project);
            project.registerReflection(projModule);
            project.children.push(projModule);
            this.oldIdToNewId = { [proj.id]: projModule.id };
            this.fromObject(projModule, proj);
            const deferred = this.deferred;
            this.deferred = [];
            for (const def of deferred) {
                def(project);
            }
            (0, assert_1.ok)(this.deferred.length === 0, "Work may not be double deferred when deserializing.");
            (0, assert_1.ok)(this.activeReflection.length === 0, "Imbalanced reflection deserialization");
        }
        this.oldIdToNewId = {};
        this.project = undefined;
        return project;
    }
    revive(source, creator) {
        if (source) {
            const revived = creator(source);
            this.fromObject(revived, source);
            return revived;
        }
    }
    reviveMany(sourceArray, creator) {
        if (sourceArray) {
            return sourceArray.map((item) => {
                const revived = creator(item);
                this.fromObject(revived, item);
                return revived;
            });
        }
    }
    reviveType(obj) {
        return this.revive(obj, (o) => this.constructType(o));
    }
    constructReflection(obj) {
        (0, assert_1.ok)(this.activeReflection.length > 0);
        const result = this.reflectionBuilders[obj.variant](this.activeReflection[this.activeReflection.length - 1], obj);
        this.oldIdToNewId[obj.id] = result.id;
        this.project.registerReflection(result);
        return result;
    }
    constructType(obj) {
        const result = this.typeBuilders[obj.type](obj, this);
        return result;
    }
    fromObject(receiver, obj) {
        if (receiver instanceof index_1.Reflection) {
            this.activeReflection.push(receiver);
        }
        receiver.fromObject(this, obj);
        for (const de of this.deserializers) {
            if (de.supports(receiver, obj)) {
                de.fromObject(receiver, obj);
            }
        }
        if (receiver instanceof index_1.Reflection) {
            this.activeReflection.pop();
        }
    }
    /**
     * Defers work until the initial pass of serialization has been completed.
     * This can be used to set up references which cannot be immediately restored.
     *
     * May only be called when deserializing.
     */
    defer(cb) {
        this.deferred.push(cb);
    }
}
exports.Deserializer = Deserializer;
