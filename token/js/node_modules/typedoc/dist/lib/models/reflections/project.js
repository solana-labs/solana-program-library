"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ProjectReflection = void 0;
const abstract_1 = require("./abstract");
const container_1 = require("./container");
const reference_1 = require("./reference");
const types_1 = require("../types");
const utils_1 = require("../../utils");
const kind_1 = require("./kind");
const comments_1 = require("../comments");
const ReflectionSymbolId_1 = require("./ReflectionSymbolId");
const map_1 = require("../../utils/map");
/**
 * A reflection that represents the root of the project.
 *
 * The project reflection acts as a global index, one may receive all reflections
 * and source files of the processed project through this reflection.
 */
class ProjectReflection extends container_1.ContainerReflection {
    constructor(name) {
        super(name, kind_1.ReflectionKind.Project);
        this.variant = "project";
        // Used to resolve references.
        this.symbolToReflectionIdMap = new map_1.StableKeyMap();
        this.reflectionIdToSymbolIdMap = new Map();
        this.reflectionIdToSymbolMap = new Map();
        // Maps a reflection ID to all reflections with it as their parent.
        this.reflectionChildren = new map_1.DefaultMap(() => []);
        /**
         * A list of all reflections within the project. DO NOT MUTATE THIS OBJECT.
         * All mutation should be done via {@link registerReflection} and {@link removeReflection}
         * to ensure that links to reflections remain valid.
         *
         * This may be replaced with a `Map<number, Reflection>` someday.
         */
        this.reflections = {};
        this.reflections[this.id] = this;
    }
    /**
     * Return whether this reflection is the root / project reflection.
     */
    isProject() {
        return true;
    }
    /**
     * Return a list of all reflections in this project of a certain kind.
     *
     * @param kind  The desired kind of reflection.
     * @returns     An array containing all reflections with the desired kind.
     */
    getReflectionsByKind(kind) {
        return Object.values(this.reflections).filter((reflection) => reflection.kindOf(kind));
    }
    /**
     * Registers the given reflection so that it can be quickly looked up by helper methods.
     * Should be called for *every* reflection added to the project.
     */
    registerReflection(reflection, symbol) {
        this.referenceGraph = undefined;
        if (reflection.parent) {
            this.reflectionChildren
                .get(reflection.parent.id)
                .push(reflection.id);
        }
        this.reflections[reflection.id] = reflection;
        if (symbol) {
            const id = new ReflectionSymbolId_1.ReflectionSymbolId(symbol);
            this.symbolToReflectionIdMap.set(id, this.symbolToReflectionIdMap.get(id) ?? reflection.id);
            this.reflectionIdToSymbolIdMap.set(reflection.id, id);
            this.reflectionIdToSymbolMap.set(reflection.id, symbol);
        }
    }
    /**
     * Removes a reflection from the documentation. Can be used by plugins to filter reflections
     * out of the generated documentation. Has no effect if the reflection is not present in the
     * project.
     */
    removeReflection(reflection) {
        // Remove the reflection...
        this._removeReflection(reflection);
        // And now try to remove references to it in the parent reflection.
        // This might not find anything if someone called removeReflection on a member of a union
        // but I think that could only be caused by a plugin doing something weird, not by a regular
        // user... so this is probably good enough for now. Reflections that live on types are
        // kind of half-real anyways.
        const parent = reflection.parent;
        parent?.traverse((child, property) => {
            if (child !== reflection) {
                return true; // Continue iteration
            }
            if (property === abstract_1.TraverseProperty.Children) {
                (0, utils_1.removeIfPresent)(parent.children, reflection);
            }
            else if (property === abstract_1.TraverseProperty.GetSignature) {
                delete parent.getSignature;
            }
            else if (property === abstract_1.TraverseProperty.IndexSignature) {
                delete parent.indexSignature;
            }
            else if (property === abstract_1.TraverseProperty.Parameters) {
                (0, utils_1.removeIfPresent)(reflection.parent.parameters, reflection);
            }
            else if (property === abstract_1.TraverseProperty.SetSignature) {
                delete parent.setSignature;
            }
            else if (property === abstract_1.TraverseProperty.Signatures) {
                (0, utils_1.removeIfPresent)(parent.signatures, reflection);
            }
            else if (property === abstract_1.TraverseProperty.TypeLiteral) {
                parent.type = new types_1.IntrinsicType("Object");
            }
            else if (property === abstract_1.TraverseProperty.TypeParameter) {
                (0, utils_1.removeIfPresent)(parent.typeParameters, reflection);
            }
            return false; // Stop iteration
        });
    }
    /**
     * Remove a reflection without updating the parent reflection to remove references to the removed reflection.
     */
    _removeReflection(reflection) {
        // Remove references pointing to this reflection
        const graph = this.getReferenceGraph();
        for (const id of graph.get(reflection.id) ?? []) {
            const ref = this.getReflectionById(id);
            if (ref) {
                this.removeReflection(ref);
            }
        }
        graph.delete(reflection.id);
        // Remove children of this reflection
        for (const childId of this.reflectionChildren.getNoInsert(reflection.id) || []) {
            const child = this.getReflectionById(childId);
            // Only remove if the child's parent is still actually this reflection.
            // This might not be the case if a plugin has moved this reflection to another parent.
            // (typedoc-plugin-merge-modules)
            if (child?.parent === reflection) {
                this._removeReflection(child);
            }
        }
        this.reflectionChildren.delete(reflection.id);
        // Remove references from the TS symbol to this reflection.
        const symbol = this.reflectionIdToSymbolMap.get(reflection.id);
        if (symbol) {
            const id = new ReflectionSymbolId_1.ReflectionSymbolId(symbol);
            if (this.symbolToReflectionIdMap.get(id) === reflection.id) {
                this.symbolToReflectionIdMap.delete(id);
            }
        }
        this.reflectionIdToSymbolIdMap.delete(reflection.id);
        delete this.reflections[reflection.id];
    }
    /**
     * Gets the reflection registered for the given reflection ID, or undefined if it is not present
     * in the project.
     */
    getReflectionById(id) {
        return this.reflections[id];
    }
    /**
     * Gets the reflection associated with the given symbol, if it exists.
     * @internal
     */
    getReflectionFromSymbol(symbol) {
        return this.getReflectionFromSymbolId(new ReflectionSymbolId_1.ReflectionSymbolId(symbol));
    }
    /**
     * Gets the reflection associated with the given symbol id, if it exists.
     * @internal
     */
    getReflectionFromSymbolId(symbolId) {
        const id = this.symbolToReflectionIdMap.get(symbolId);
        if (typeof id === "number") {
            return this.getReflectionById(id);
        }
    }
    /** @internal */
    getSymbolIdFromReflection(reflection) {
        return this.reflectionIdToSymbolIdMap.get(reflection.id);
    }
    /** @internal */
    registerSymbolId(reflection, id) {
        this.reflectionIdToSymbolIdMap.set(reflection.id, id);
        if (!this.symbolToReflectionIdMap.has(id)) {
            this.symbolToReflectionIdMap.set(id, reflection.id);
        }
    }
    /**
     * THIS MAY NOT BE USED AFTER CONVERSION HAS FINISHED.
     * @internal
     */
    getSymbolFromReflection(reflection) {
        return this.reflectionIdToSymbolMap.get(reflection.id);
    }
    getReferenceGraph() {
        if (!this.referenceGraph) {
            this.referenceGraph = new Map();
            for (const ref of Object.values(this.reflections)) {
                if (ref instanceof reference_1.ReferenceReflection) {
                    const target = ref.tryGetTargetReflection();
                    if (target) {
                        const refs = this.referenceGraph.get(target.id) ?? [];
                        refs.push(ref.id);
                        this.referenceGraph.set(target.id, refs);
                    }
                }
            }
        }
        return this.referenceGraph;
    }
    toObject(serializer) {
        const symbolIdMap = {};
        this.reflectionIdToSymbolIdMap.forEach((sid, id) => {
            symbolIdMap[id] = sid.toObject(serializer);
        });
        return {
            ...super.toObject(serializer),
            variant: this.variant,
            packageName: this.packageName,
            packageVersion: this.packageVersion,
            readme: comments_1.Comment.serializeDisplayParts(serializer, this.readme),
            symbolIdMap,
        };
    }
    fromObject(de, obj) {
        super.fromObject(de, obj);
        // If updating this, also check the block in DeclarationReflection.fromObject.
        this.packageName = obj.packageName;
        this.packageVersion = obj.packageVersion;
        if (obj.readme) {
            this.readme = comments_1.Comment.deserializeDisplayParts(de, obj.readme);
        }
        de.defer(() => {
            for (const [id, sid] of Object.entries(obj.symbolIdMap || {})) {
                const refl = this.getReflectionById(de.oldIdToNewId[+id] ?? -1);
                if (refl) {
                    this.registerSymbolId(refl, new ReflectionSymbolId_1.ReflectionSymbolId(sid));
                }
                else {
                    de.logger.warn(`Serialized project contained a reflection with id ${id} but it was not present in deserialized project.`);
                }
            }
        });
    }
}
exports.ProjectReflection = ProjectReflection;
