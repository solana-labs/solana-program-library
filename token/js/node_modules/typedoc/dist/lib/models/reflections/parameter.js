"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ParameterReflection = void 0;
const types_1 = require("../types");
const abstract_1 = require("./abstract");
class ParameterReflection extends abstract_1.Reflection {
    constructor() {
        super(...arguments);
        this.variant = "param";
    }
    traverse(callback) {
        if (this.type instanceof types_1.ReflectionType) {
            if (callback(this.type.declaration, abstract_1.TraverseProperty.TypeLiteral) === false) {
                return;
            }
        }
    }
    /**
     * Return a string representation of this reflection.
     */
    toString() {
        return super.toString() + (this.type ? ":" + this.type.toString() : "");
    }
    toObject(serializer) {
        return {
            ...super.toObject(serializer),
            variant: this.variant,
            type: serializer.toObject(this.type),
            defaultValue: this.defaultValue,
        };
    }
    fromObject(de, obj) {
        super.fromObject(de, obj);
        this.type = de.reviveType(obj.type);
        this.defaultValue = obj.defaultValue;
    }
}
exports.ParameterReflection = ParameterReflection;
