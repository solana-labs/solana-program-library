"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.LinkResolverPlugin = void 0;
const components_1 = require("../components");
const converter_events_1 = require("../converter-events");
const utils_1 = require("../../utils");
const models_1 = require("../../models");
const reflections_1 = require("../../utils/reflections");
const application_events_1 = require("../../application-events");
/**
 * A plugin that resolves `{@link Foo}` tags.
 */
let LinkResolverPlugin = class LinkResolverPlugin extends components_1.ConverterComponent {
    initialize() {
        super.initialize();
        this.owner.on(converter_events_1.ConverterEvents.RESOLVE_END, this.onResolve, this, -300);
        this.application.on(application_events_1.ApplicationEvents.REVIVE, this.resolveLinks, this, -300);
    }
    onResolve(context) {
        this.resolveLinks(context.project);
    }
    resolveLinks(project) {
        for (const reflection of Object.values(project.reflections)) {
            if (reflection.comment) {
                this.owner.resolveLinks(reflection.comment, reflection);
            }
            if (reflection instanceof models_1.DeclarationReflection &&
                reflection.readme) {
                reflection.readme = this.owner.resolveLinks(reflection.readme, reflection);
            }
        }
        if (project.readme) {
            project.readme = this.owner.resolveLinks(project.readme, project);
        }
        for (const { type, owner } of (0, reflections_1.discoverAllReferenceTypes)(project, false)) {
            if (!type.reflection) {
                const resolveResult = this.owner.resolveExternalLink(type.toDeclarationReference(), owner, undefined, type.symbolId);
                switch (typeof resolveResult) {
                    case "string":
                        type.externalUrl = resolveResult;
                        break;
                    case "object":
                        type.externalUrl = resolveResult.target;
                        break;
                }
            }
        }
    }
};
__decorate([
    (0, utils_1.BindOption)("validation")
], LinkResolverPlugin.prototype, "validation", void 0);
LinkResolverPlugin = __decorate([
    (0, components_1.Component)({ name: "link-resolver" })
], LinkResolverPlugin);
exports.LinkResolverPlugin = LinkResolverPlugin;
