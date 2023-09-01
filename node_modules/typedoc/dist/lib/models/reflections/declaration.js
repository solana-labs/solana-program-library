"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.DeclarationReflection = exports.ConversionFlags = void 0;
const types_1 = require("../types");
const abstract_1 = require("./abstract");
const container_1 = require("./container");
const comments_1 = require("../comments");
const file_1 = require("../sources/file");
const ReflectionSymbolId_1 = require("./ReflectionSymbolId");
const kind_1 = require("./kind");
/**
 * @internal
 */
var ConversionFlags;
(function (ConversionFlags) {
    ConversionFlags[ConversionFlags["None"] = 0] = "None";
    ConversionFlags[ConversionFlags["VariableOrPropertySource"] = 1] = "VariableOrPropertySource";
})(ConversionFlags = exports.ConversionFlags || (exports.ConversionFlags = {}));
/**
 * A reflection that represents a single declaration emitted by the TypeScript compiler.
 *
 * All parts of a project are represented by DeclarationReflection instances. The actual
 * kind of a reflection is stored in its ´kind´ member.
 */
class DeclarationReflection extends container_1.ContainerReflection {
    constructor() {
        super(...arguments);
        this.variant = "declaration";
        /**
         * Flags for information about a reflection which is needed solely during conversion.
         * @internal
         */
        this.conversionFlags = ConversionFlags.None;
    }
    isDeclaration() {
        return true;
    }
    hasGetterOrSetter() {
        return !!this.getSignature || !!this.setSignature;
    }
    getAllSignatures() {
        let result = [];
        if (this.signatures) {
            result = result.concat(this.signatures);
        }
        if (this.indexSignature) {
            result.push(this.indexSignature);
        }
        if (this.getSignature) {
            result.push(this.getSignature);
        }
        if (this.setSignature) {
            result.push(this.setSignature);
        }
        return result;
    }
    /** @internal */
    getNonIndexSignatures() {
        return [].concat(this.signatures ?? [], this.setSignature ?? [], this.getSignature ?? []);
    }
    traverse(callback) {
        for (const parameter of this.typeParameters?.slice() || []) {
            if (callback(parameter, abstract_1.TraverseProperty.TypeParameter) === false) {
                return;
            }
        }
        if (this.type instanceof types_1.ReflectionType) {
            if (callback(this.type.declaration, abstract_1.TraverseProperty.TypeLiteral) === false) {
                return;
            }
        }
        for (const signature of this.signatures?.slice() || []) {
            if (callback(signature, abstract_1.TraverseProperty.Signatures) === false) {
                return;
            }
        }
        if (this.indexSignature) {
            if (callback(this.indexSignature, abstract_1.TraverseProperty.IndexSignature) === false) {
                return;
            }
        }
        if (this.getSignature) {
            if (callback(this.getSignature, abstract_1.TraverseProperty.GetSignature) ===
                false) {
                return;
            }
        }
        if (this.setSignature) {
            if (callback(this.setSignature, abstract_1.TraverseProperty.SetSignature) ===
                false) {
                return;
            }
        }
        super.traverse(callback);
    }
    /**
     * Return a string representation of this reflection.
     */
    toString() {
        let result = super.toString();
        if (this.typeParameters) {
            const parameters = this.typeParameters.map((parameter) => parameter.name);
            result += "<" + parameters.join(", ") + ">";
        }
        if (this.type) {
            result += ":" + this.type.toString();
        }
        return result;
    }
    toObject(serializer) {
        return {
            ...super.toObject(serializer),
            variant: this.variant,
            packageVersion: this.packageVersion,
            sources: serializer.toObjectsOptional(this.sources),
            relevanceBoost: this.relevanceBoost === 1 ? undefined : this.relevanceBoost,
            typeParameters: serializer.toObjectsOptional(this.typeParameters),
            type: serializer.toObject(this.type),
            signatures: serializer.toObjectsOptional(this.signatures),
            indexSignature: serializer.toObject(this.indexSignature),
            getSignature: serializer.toObject(this.getSignature),
            setSignature: serializer.toObject(this.setSignature),
            defaultValue: this.defaultValue,
            overwrites: serializer.toObject(this.overwrites),
            inheritedFrom: serializer.toObject(this.inheritedFrom),
            implementationOf: serializer.toObject(this.implementationOf),
            extendedTypes: serializer.toObjectsOptional(this.extendedTypes),
            extendedBy: serializer.toObjectsOptional(this.extendedBy),
            implementedTypes: serializer.toObjectsOptional(this.implementedTypes),
            implementedBy: serializer.toObjectsOptional(this.implementedBy),
        };
    }
    fromObject(de, obj) {
        super.fromObject(de, obj);
        // This happens when merging multiple projects together.
        // If updating this, also check ProjectReflection.fromObject.
        if (obj.variant === "project") {
            this.kind = kind_1.ReflectionKind.Module;
            this.packageVersion = obj.packageVersion;
            if (obj.readme) {
                this.readme = comments_1.Comment.deserializeDisplayParts(de, obj.readme);
            }
            de.defer(() => {
                for (const [id, sid] of Object.entries(obj.symbolIdMap || {})) {
                    const refl = this.project.getReflectionById(de.oldIdToNewId[+id] ?? -1);
                    if (refl) {
                        this.project.registerSymbolId(refl, new ReflectionSymbolId_1.ReflectionSymbolId(sid));
                    }
                    else {
                        de.logger.warn(`Serialized project contained a reflection with id ${id} but it was not present in deserialized project.`);
                    }
                }
            });
            return;
        }
        this.packageVersion = obj.packageVersion;
        this.sources = de.reviveMany(obj.sources, (src) => new file_1.SourceReference(src.fileName, src.line, src.character));
        this.relevanceBoost = obj.relevanceBoost;
        this.typeParameters = de.reviveMany(obj.typeParameters, (tp) => de.constructReflection(tp));
        this.type = de.revive(obj.type, (t) => de.constructType(t));
        this.signatures = de.reviveMany(obj.signatures, (r) => de.constructReflection(r));
        this.indexSignature = de.revive(obj.indexSignature, (r) => de.constructReflection(r));
        this.getSignature = de.revive(obj.getSignature, (r) => de.constructReflection(r));
        this.setSignature = de.revive(obj.setSignature, (r) => de.constructReflection(r));
        this.defaultValue = obj.defaultValue;
        this.overwrites = de.reviveType(obj.overwrites);
        this.inheritedFrom = de.reviveType(obj.inheritedFrom);
        this.implementationOf = de.reviveType(obj.implementationOf);
        this.extendedTypes = de.reviveMany(obj.extendedTypes, (t) => de.reviveType(t));
        this.extendedBy = de.reviveMany(obj.extendedBy, (t) => de.reviveType(t));
        this.implementedTypes = de.reviveMany(obj.implementedTypes, (t) => de.reviveType(t));
        this.implementedBy = de.reviveMany(obj.implementedBy, (t) => de.reviveType(t));
    }
}
exports.DeclarationReflection = DeclarationReflection;
