"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.TypeParameterReflection = exports.VarianceModifier = void 0;
const abstract_1 = require("./abstract");
const kind_1 = require("./kind");
/**
 * Modifier flags for type parameters, added in TS 4.7
 * @enum
 */
exports.VarianceModifier = {
    in: "in",
    out: "out",
    inOut: "in out",
};
class TypeParameterReflection extends abstract_1.Reflection {
    constructor(name, parent, varianceModifier) {
        super(name, kind_1.ReflectionKind.TypeParameter, parent);
        this.variant = "typeParam";
        this.varianceModifier = varianceModifier;
    }
    toObject(serializer) {
        return {
            ...super.toObject(serializer),
            variant: this.variant,
            type: serializer.toObject(this.type),
            default: serializer.toObject(this.default),
            varianceModifier: this.varianceModifier,
        };
    }
    fromObject(de, obj) {
        super.fromObject(de, obj);
        this.type = de.reviveType(obj.type);
        this.default = de.reviveType(obj.default);
        this.varianceModifier = obj.varianceModifier;
    }
    traverse(_callback) {
        // do nothing, no child reflections.
    }
}
exports.TypeParameterReflection = TypeParameterReflection;
