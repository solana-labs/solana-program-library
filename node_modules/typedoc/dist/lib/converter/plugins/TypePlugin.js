"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.TypePlugin = void 0;
const index_1 = require("../../models/reflections/index");
const types_1 = require("../../models/types");
const components_1 = require("../components");
const converter_1 = require("../converter");
const application_events_1 = require("../../application-events");
/**
 * Responsible for adding `implementedBy` / `implementedFrom`
 */
let TypePlugin = class TypePlugin extends components_1.ConverterComponent {
    constructor() {
        super(...arguments);
        this.reflections = new Set();
    }
    /**
     * Create a new TypeHandler instance.
     */
    initialize() {
        this.listenTo(this.owner, {
            [converter_1.Converter.EVENT_RESOLVE]: this.onResolve,
            [converter_1.Converter.EVENT_RESOLVE_END]: this.onResolveEnd,
            [converter_1.Converter.EVENT_END]: () => this.reflections.clear(),
        });
        this.listenTo(this.application, {
            [application_events_1.ApplicationEvents.REVIVE]: this.onRevive,
        });
    }
    onRevive(project) {
        for (const refl of Object.values(project.reflections)) {
            this.resolve(project, refl);
        }
        this.finishResolve(project);
        this.reflections.clear();
    }
    onResolve(context, reflection) {
        this.resolve(context.project, reflection);
    }
    resolve(project, reflection) {
        if (!(reflection instanceof index_1.DeclarationReflection))
            return;
        if (reflection.kindOf(index_1.ReflectionKind.ClassOrInterface)) {
            this.postpone(reflection);
            walk(reflection.implementedTypes, (target) => {
                this.postpone(target);
                if (!target.implementedBy) {
                    target.implementedBy = [];
                }
                target.implementedBy.push(types_1.ReferenceType.createResolvedReference(reflection.name, reflection, project));
            });
            walk(reflection.extendedTypes, (target) => {
                this.postpone(target);
                if (!target.extendedBy) {
                    target.extendedBy = [];
                }
                target.extendedBy.push(types_1.ReferenceType.createResolvedReference(reflection.name, reflection, project));
            });
        }
        function walk(types, callback) {
            if (!types) {
                return;
            }
            types.forEach((type) => {
                if (!(type instanceof types_1.ReferenceType)) {
                    return;
                }
                if (!type.reflection ||
                    !(type.reflection instanceof index_1.DeclarationReflection)) {
                    return;
                }
                callback(type.reflection);
            });
        }
    }
    postpone(reflection) {
        this.reflections.add(reflection);
    }
    onResolveEnd(context) {
        this.finishResolve(context.project);
    }
    finishResolve(project) {
        this.reflections.forEach((reflection) => {
            if (reflection.implementedBy) {
                reflection.implementedBy.sort((a, b) => {
                    if (a.name === b.name) {
                        return 0;
                    }
                    return a.name > b.name ? 1 : -1;
                });
            }
            let root;
            let hierarchy;
            function push(types) {
                const level = { types: types };
                if (hierarchy) {
                    hierarchy.next = level;
                    hierarchy = level;
                }
                else {
                    root = hierarchy = level;
                }
            }
            if (reflection.extendedTypes) {
                push(reflection.extendedTypes);
            }
            push([
                types_1.ReferenceType.createResolvedReference(reflection.name, reflection, project),
            ]);
            hierarchy.isTarget = true;
            if (reflection.extendedBy) {
                push(reflection.extendedBy);
            }
            reflection.typeHierarchy = root;
        });
    }
};
TypePlugin = __decorate([
    (0, components_1.Component)({ name: "type" })
], TypePlugin);
exports.TypePlugin = TypePlugin;
