"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.SignatureReflection = void 0;
const types_1 = require("../types");
const abstract_1 = require("./abstract");
const file_1 = require("../sources/file");
class SignatureReflection extends abstract_1.Reflection {
    constructor(name, kind, parent) {
        super(name, kind, parent);
        this.variant = "signature";
    }
    traverse(callback) {
        if (this.type instanceof types_1.ReflectionType) {
            if (callback(this.type.declaration, abstract_1.TraverseProperty.TypeLiteral) === false) {
                return;
            }
        }
        for (const parameter of this.typeParameters?.slice() || []) {
            if (callback(parameter, abstract_1.TraverseProperty.TypeParameter) === false) {
                return;
            }
        }
        for (const parameter of this.parameters?.slice() || []) {
            if (callback(parameter, abstract_1.TraverseProperty.Parameters) === false) {
                return;
            }
        }
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
            sources: serializer.toObjectsOptional(this.sources),
            typeParameter: serializer.toObjectsOptional(this.typeParameters),
            parameters: serializer.toObjectsOptional(this.parameters),
            type: serializer.toObject(this.type),
            overwrites: serializer.toObject(this.overwrites),
            inheritedFrom: serializer.toObject(this.inheritedFrom),
            implementationOf: serializer.toObject(this.implementationOf),
        };
    }
    fromObject(de, obj) {
        super.fromObject(de, obj);
        this.sources = de.reviveMany(obj.sources, (t) => new file_1.SourceReference(t.fileName, t.line, t.character));
        this.typeParameters = de.reviveMany(obj.typeParameter, (t) => de.constructReflection(t));
        this.parameters = de.reviveMany(obj.parameters, (t) => de.constructReflection(t));
        this.type = de.reviveType(obj.type);
        this.overwrites = de.reviveType(obj.overwrites);
        this.inheritedFrom = de.reviveType(obj.inheritedFrom);
        this.implementationOf = de.reviveType(obj.implementationOf);
    }
}
exports.SignatureReflection = SignatureReflection;
