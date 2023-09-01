"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ReflectionKind = void 0;
/**
 * Defines the available reflection kinds.
 */
var ReflectionKind;
(function (ReflectionKind) {
    ReflectionKind[ReflectionKind["Project"] = 1] = "Project";
    ReflectionKind[ReflectionKind["Module"] = 2] = "Module";
    ReflectionKind[ReflectionKind["Namespace"] = 4] = "Namespace";
    ReflectionKind[ReflectionKind["Enum"] = 8] = "Enum";
    ReflectionKind[ReflectionKind["EnumMember"] = 16] = "EnumMember";
    ReflectionKind[ReflectionKind["Variable"] = 32] = "Variable";
    ReflectionKind[ReflectionKind["Function"] = 64] = "Function";
    ReflectionKind[ReflectionKind["Class"] = 128] = "Class";
    ReflectionKind[ReflectionKind["Interface"] = 256] = "Interface";
    ReflectionKind[ReflectionKind["Constructor"] = 512] = "Constructor";
    ReflectionKind[ReflectionKind["Property"] = 1024] = "Property";
    ReflectionKind[ReflectionKind["Method"] = 2048] = "Method";
    ReflectionKind[ReflectionKind["CallSignature"] = 4096] = "CallSignature";
    ReflectionKind[ReflectionKind["IndexSignature"] = 8192] = "IndexSignature";
    ReflectionKind[ReflectionKind["ConstructorSignature"] = 16384] = "ConstructorSignature";
    ReflectionKind[ReflectionKind["Parameter"] = 32768] = "Parameter";
    ReflectionKind[ReflectionKind["TypeLiteral"] = 65536] = "TypeLiteral";
    ReflectionKind[ReflectionKind["TypeParameter"] = 131072] = "TypeParameter";
    ReflectionKind[ReflectionKind["Accessor"] = 262144] = "Accessor";
    ReflectionKind[ReflectionKind["GetSignature"] = 524288] = "GetSignature";
    ReflectionKind[ReflectionKind["SetSignature"] = 1048576] = "SetSignature";
    /** @deprecated will be removed in v0.25, not used */
    ReflectionKind[ReflectionKind["ObjectLiteral"] = 2097152] = "ObjectLiteral";
    ReflectionKind[ReflectionKind["TypeAlias"] = 4194304] = "TypeAlias";
    ReflectionKind[ReflectionKind["Reference"] = 8388608] = "Reference";
})(ReflectionKind = exports.ReflectionKind || (exports.ReflectionKind = {}));
(function (ReflectionKind) {
    ReflectionKind.All = ReflectionKind.Reference * 2 - 1;
    /** @internal */
    ReflectionKind.ClassOrInterface = ReflectionKind.Class | ReflectionKind.Interface;
    /** @internal */
    ReflectionKind.VariableOrProperty = ReflectionKind.Variable | ReflectionKind.Property;
    /** @internal */
    ReflectionKind.FunctionOrMethod = ReflectionKind.Function | ReflectionKind.Method;
    /** @internal */
    ReflectionKind.ClassMember = ReflectionKind.Accessor |
        ReflectionKind.Constructor |
        ReflectionKind.Method |
        ReflectionKind.Property;
    /** @internal */
    ReflectionKind.SomeSignature = ReflectionKind.CallSignature |
        ReflectionKind.IndexSignature |
        ReflectionKind.ConstructorSignature |
        ReflectionKind.GetSignature |
        ReflectionKind.SetSignature;
    /** @internal */
    ReflectionKind.SomeModule = ReflectionKind.Namespace | ReflectionKind.Module;
    /** @internal */
    ReflectionKind.SomeType = ReflectionKind.Interface |
        ReflectionKind.TypeLiteral |
        ReflectionKind.TypeParameter |
        ReflectionKind.TypeAlias;
    /** @internal */
    ReflectionKind.SomeValue = ReflectionKind.Variable |
        ReflectionKind.Function |
        ReflectionKind.ObjectLiteral;
    /** @internal */
    ReflectionKind.SomeMember = ReflectionKind.EnumMember |
        ReflectionKind.Property |
        ReflectionKind.Method |
        ReflectionKind.Accessor;
    /** @internal */
    ReflectionKind.SomeExport = ReflectionKind.Module |
        ReflectionKind.Namespace |
        ReflectionKind.Enum |
        ReflectionKind.Variable |
        ReflectionKind.Function |
        ReflectionKind.Class |
        ReflectionKind.Interface |
        ReflectionKind.TypeAlias |
        ReflectionKind.Reference;
    /** @internal */
    ReflectionKind.ExportContainer = ReflectionKind.SomeModule | ReflectionKind.Project;
    /** @internal */
    ReflectionKind.Inheritable = ReflectionKind.Accessor |
        ReflectionKind.IndexSignature |
        ReflectionKind.Property |
        ReflectionKind.Method |
        ReflectionKind.Constructor;
    /** @internal */
    ReflectionKind.ContainsCallSignatures = ReflectionKind.Constructor |
        ReflectionKind.Function |
        ReflectionKind.Method;
    /**
     * Note: This does not include Class/Interface, even though they technically could contain index signatures
     * @internal
     */
    ReflectionKind.SignatureContainer = ReflectionKind.ContainsCallSignatures | ReflectionKind.Accessor;
    const SINGULARS = {
        [ReflectionKind.Enum]: "Enumeration",
        [ReflectionKind.EnumMember]: "Enumeration Member",
    };
    const PLURALS = {
        [ReflectionKind.Class]: "Classes",
        [ReflectionKind.Property]: "Properties",
        [ReflectionKind.Enum]: "Enumerations",
        [ReflectionKind.EnumMember]: "Enumeration Members",
        [ReflectionKind.TypeAlias]: "Type Aliases",
    };
    function singularString(kind) {
        if (kind in SINGULARS) {
            return SINGULARS[kind];
        }
        else {
            return getKindString(kind);
        }
    }
    ReflectionKind.singularString = singularString;
    function pluralString(kind) {
        if (kind in PLURALS) {
            return PLURALS[kind];
        }
        else {
            return getKindString(kind) + "s";
        }
    }
    ReflectionKind.pluralString = pluralString;
    function classString(kind) {
        return `tsd-kind-${ReflectionKind[kind]
            .replace(/(.)([A-Z])/g, (_m, a, b) => `${a}-${b}`)
            .toLowerCase()}`;
    }
    ReflectionKind.classString = classString;
})(ReflectionKind = exports.ReflectionKind || (exports.ReflectionKind = {}));
function getKindString(kind) {
    return ReflectionKind[kind].replace(/(.)([A-Z])/g, (_m, a, b) => a + " " + b.toLowerCase());
}
