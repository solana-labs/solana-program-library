"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Reflection = exports.TraverseProperty = exports.ReflectionFlags = exports.ReflectionFlag = exports.resetReflectionID = void 0;
const assert_1 = require("assert");
const comment_1 = require("../comments/comment");
const utils_1 = require("./utils");
const kind_1 = require("./kind");
/**
 * Current reflection id.
 */
let REFLECTION_ID = 0;
/**
 * Reset the reflection id.
 *
 * Used by the test cases to ensure the reflection ids won't change between runs.
 */
function resetReflectionID() {
    REFLECTION_ID = 0;
}
exports.resetReflectionID = resetReflectionID;
var ReflectionFlag;
(function (ReflectionFlag) {
    ReflectionFlag[ReflectionFlag["None"] = 0] = "None";
    ReflectionFlag[ReflectionFlag["Private"] = 1] = "Private";
    ReflectionFlag[ReflectionFlag["Protected"] = 2] = "Protected";
    ReflectionFlag[ReflectionFlag["Public"] = 4] = "Public";
    ReflectionFlag[ReflectionFlag["Static"] = 8] = "Static";
    ReflectionFlag[ReflectionFlag["ExportAssignment"] = 16] = "ExportAssignment";
    ReflectionFlag[ReflectionFlag["External"] = 32] = "External";
    ReflectionFlag[ReflectionFlag["Optional"] = 64] = "Optional";
    ReflectionFlag[ReflectionFlag["DefaultValue"] = 128] = "DefaultValue";
    ReflectionFlag[ReflectionFlag["Rest"] = 256] = "Rest";
    ReflectionFlag[ReflectionFlag["Abstract"] = 512] = "Abstract";
    ReflectionFlag[ReflectionFlag["Const"] = 1024] = "Const";
    ReflectionFlag[ReflectionFlag["Let"] = 2048] = "Let";
    ReflectionFlag[ReflectionFlag["Readonly"] = 4096] = "Readonly";
})(ReflectionFlag = exports.ReflectionFlag || (exports.ReflectionFlag = {}));
const relevantFlags = [
    ReflectionFlag.Private,
    ReflectionFlag.Protected,
    ReflectionFlag.Static,
    ReflectionFlag.ExportAssignment,
    ReflectionFlag.Optional,
    ReflectionFlag.DefaultValue,
    ReflectionFlag.Rest,
    ReflectionFlag.Abstract,
    ReflectionFlag.Const,
    ReflectionFlag.Readonly,
];
/**
 * This must extend Array in order to work with Handlebar's each helper.
 */
class ReflectionFlags extends Array {
    constructor() {
        super(...arguments);
        this.flags = ReflectionFlag.None;
    }
    hasFlag(flag) {
        return (flag & this.flags) !== 0;
    }
    /**
     * Is this a private member?
     */
    get isPrivate() {
        return this.hasFlag(ReflectionFlag.Private);
    }
    /**
     * Is this a protected member?
     */
    get isProtected() {
        return this.hasFlag(ReflectionFlag.Protected);
    }
    /**
     * Is this a public member?
     */
    get isPublic() {
        return this.hasFlag(ReflectionFlag.Public);
    }
    /**
     * Is this a static member?
     */
    get isStatic() {
        return this.hasFlag(ReflectionFlag.Static);
    }
    /**
     * Is this a declaration from an external document?
     */
    get isExternal() {
        return this.hasFlag(ReflectionFlag.External);
    }
    /**
     * Whether this reflection is an optional component or not.
     *
     * Applies to function parameters and object members.
     */
    get isOptional() {
        return this.hasFlag(ReflectionFlag.Optional);
    }
    /**
     * Whether it's a rest parameter, like `foo(...params);`.
     */
    get isRest() {
        return this.hasFlag(ReflectionFlag.Rest);
    }
    get hasExportAssignment() {
        return this.hasFlag(ReflectionFlag.ExportAssignment);
    }
    get isAbstract() {
        return this.hasFlag(ReflectionFlag.Abstract);
    }
    get isConst() {
        return this.hasFlag(ReflectionFlag.Const);
    }
    get isReadonly() {
        return this.hasFlag(ReflectionFlag.Readonly);
    }
    setFlag(flag, set) {
        switch (flag) {
            case ReflectionFlag.Private:
                this.setSingleFlag(ReflectionFlag.Private, set);
                if (set) {
                    this.setFlag(ReflectionFlag.Protected, false);
                    this.setFlag(ReflectionFlag.Public, false);
                }
                break;
            case ReflectionFlag.Protected:
                this.setSingleFlag(ReflectionFlag.Protected, set);
                if (set) {
                    this.setFlag(ReflectionFlag.Private, false);
                    this.setFlag(ReflectionFlag.Public, false);
                }
                break;
            case ReflectionFlag.Public:
                this.setSingleFlag(ReflectionFlag.Public, set);
                if (set) {
                    this.setFlag(ReflectionFlag.Private, false);
                    this.setFlag(ReflectionFlag.Protected, false);
                }
                break;
            default:
                this.setSingleFlag(flag, set);
        }
    }
    setSingleFlag(flag, set) {
        const name = ReflectionFlag[flag].replace(/(.)([A-Z])/g, (_m, a, b) => a + " " + b.toLowerCase());
        if (!set && this.hasFlag(flag)) {
            if (relevantFlags.includes(flag)) {
                this.splice(this.indexOf(name), 1);
            }
            this.flags ^= flag;
        }
        else if (set && !this.hasFlag(flag)) {
            if (relevantFlags.includes(flag)) {
                this.push(name);
            }
            this.flags |= flag;
        }
    }
    toObject() {
        return Object.fromEntries(ReflectionFlags.serializedFlags
            .filter((flag) => this[flag])
            .map((flag) => [flag, true]));
    }
    fromObject(obj) {
        for (const key of Object.keys(obj)) {
            const flagName = key.substring(2); // isPublic => Public
            if (flagName in ReflectionFlag) {
                this.setFlag(ReflectionFlag[flagName], true);
            }
        }
    }
}
ReflectionFlags.serializedFlags = [
    "isPrivate",
    "isProtected",
    "isPublic",
    "isStatic",
    "isExternal",
    "isOptional",
    "isRest",
    "hasExportAssignment",
    "isAbstract",
    "isConst",
    "isReadonly",
];
exports.ReflectionFlags = ReflectionFlags;
var TraverseProperty;
(function (TraverseProperty) {
    TraverseProperty[TraverseProperty["Children"] = 0] = "Children";
    TraverseProperty[TraverseProperty["Parameters"] = 1] = "Parameters";
    TraverseProperty[TraverseProperty["TypeLiteral"] = 2] = "TypeLiteral";
    TraverseProperty[TraverseProperty["TypeParameter"] = 3] = "TypeParameter";
    TraverseProperty[TraverseProperty["Signatures"] = 4] = "Signatures";
    TraverseProperty[TraverseProperty["IndexSignature"] = 5] = "IndexSignature";
    TraverseProperty[TraverseProperty["GetSignature"] = 6] = "GetSignature";
    TraverseProperty[TraverseProperty["SetSignature"] = 7] = "SetSignature";
})(TraverseProperty = exports.TraverseProperty || (exports.TraverseProperty = {}));
/**
 * Base class for all reflection classes.
 *
 * While generating a documentation, TypeDoc generates an instance of {@link ProjectReflection}
 * as the root for all reflections within the project. All other reflections are represented
 * by the {@link DeclarationReflection} class.
 *
 * This base class exposes the basic properties one may use to traverse the reflection tree.
 * You can use the {@link ContainerReflection.children} and {@link parent} properties to walk the tree. The {@link ContainerReflection.groups} property
 * contains a list of all children grouped and sorted for rendering.
 */
class Reflection {
    get project() {
        if (this.isProject())
            return this;
        (0, assert_1.ok)(this.parent, "Tried to get the project on a reflection not in a project");
        return this.parent.project;
    }
    constructor(name, kind, parent) {
        this.flags = new ReflectionFlags();
        this.id = REFLECTION_ID++;
        this.parent = parent;
        this.name = name;
        this.kind = kind;
        // If our parent is external, we are too.
        if (parent?.flags.isExternal) {
            this.setFlag(ReflectionFlag.External);
        }
    }
    /**
     * Test whether this reflection is of the given kind.
     */
    kindOf(kind) {
        const kindArray = Array.isArray(kind) ? kind : [kind];
        return kindArray.some((kind) => (this.kind & kind) !== 0);
    }
    /**
     * Return the full name of this reflection. Intended for use in debugging. For log messages
     * intended to be displayed to the user for them to fix, prefer {@link getFriendlyFullName} instead.
     *
     * The full name contains the name of this reflection and the names of all parent reflections.
     *
     * @param separator  Separator used to join the names of the reflections.
     * @returns The full name of this reflection.
     */
    getFullName(separator = ".") {
        if (this.parent && !this.parent.isProject()) {
            return this.parent.getFullName(separator) + separator + this.name;
        }
        else {
            return this.name;
        }
    }
    /**
     * Return the full name of this reflection, with signature names dropped if possible without
     * introducing ambiguity in the name.
     */
    getFriendlyFullName() {
        if (this.parent && !this.parent.isProject()) {
            if (this.kindOf(kind_1.ReflectionKind.ConstructorSignature |
                kind_1.ReflectionKind.CallSignature |
                kind_1.ReflectionKind.GetSignature |
                kind_1.ReflectionKind.SetSignature)) {
                return this.parent.getFriendlyFullName();
            }
            return this.parent.getFriendlyFullName() + "." + this.name;
        }
        else {
            return this.name;
        }
    }
    /**
     * Set a flag on this reflection.
     */
    setFlag(flag, value = true) {
        this.flags.setFlag(flag, value);
    }
    /**
     * Return an url safe alias for this reflection.
     */
    getAlias() {
        if (!this._alias) {
            let alias = this.name.replace(/\W/g, "_");
            if (alias === "") {
                alias = "reflection-" + this.id;
            }
            // NTFS/ExFAT use uppercase, so we will too. It probably won't matter
            // in this case since names will generally be valid identifiers, but to be safe...
            const upperAlias = alias.toUpperCase();
            let target = this;
            while (target.parent && !target.hasOwnDocument) {
                target = target.parent;
            }
            target._aliases || (target._aliases = new Map());
            let suffix = "";
            if (!target._aliases.has(upperAlias)) {
                target._aliases.set(upperAlias, 1);
            }
            else {
                const count = target._aliases.get(upperAlias);
                suffix = "-" + count.toString();
                target._aliases.set(upperAlias, count + 1);
            }
            alias += suffix;
            this._alias = alias;
        }
        return this._alias;
    }
    /**
     * Has this reflection a visible comment?
     *
     * @returns TRUE when this reflection has a visible comment.
     */
    hasComment() {
        return this.comment ? this.comment.hasVisibleComponent() : false;
    }
    hasGetterOrSetter() {
        return false;
    }
    /**
     * Return a child by its name.
     *
     * @param names The name hierarchy of the child to look for.
     * @returns The found child or undefined.
     */
    getChildByName(arg) {
        const names = Array.isArray(arg)
            ? arg
            : (0, utils_1.splitUnquotedString)(arg, ".");
        const name = names[0];
        let result;
        this.traverse((child) => {
            if (child.name === name) {
                if (names.length <= 1) {
                    result = child;
                }
                else {
                    result = child.getChildByName(names.slice(1));
                }
                return false;
            }
            return true;
        });
        return result;
    }
    /**
     * Return whether this reflection is the root / project reflection.
     */
    isProject() {
        return false;
    }
    isDeclaration() {
        return false;
    }
    /**
     * Check if this reflection or any of its parents have been marked with the `@deprecated` tag.
     */
    isDeprecated() {
        if (this.comment?.getTag("@deprecated")) {
            return true;
        }
        return this.parent?.isDeprecated() ?? false;
    }
    /**
     * Return a string representation of this reflection.
     */
    toString() {
        return kind_1.ReflectionKind[this.kind] + " " + this.name;
    }
    /**
     * Return a string representation of this reflection and all of its children.
     *
     * @param indent  Used internally to indent child reflections.
     */
    toStringHierarchy(indent = "") {
        const lines = [indent + this.toString()];
        indent += "  ";
        this.traverse((child) => {
            lines.push(child.toStringHierarchy(indent));
            return true;
        });
        return lines.join("\n");
    }
    toObject(serializer) {
        return {
            id: this.id,
            name: this.name,
            variant: this.variant,
            kind: this.kind,
            flags: this.flags.toObject(),
            comment: this.comment && !this.comment.isEmpty()
                ? serializer.toObject(this.comment)
                : undefined,
        };
    }
    fromObject(de, obj) {
        // DO NOT copy id from obj. When deserializing reflections
        // they should be given new ids since they belong to a different project.
        this.name = obj.name;
        // Skip copying variant, we know it's already the correct value because the deserializer
        // will construct the correct class type.
        this.kind = obj.kind;
        this.flags.fromObject(obj.flags);
        // Parent is set during construction, so we don't need to do it here.
        this.comment = de.revive(obj.comment, () => new comment_1.Comment());
        // url, anchor, hasOwnDocument, _alias, _aliases are set during rendering and only relevant during render.
        // It doesn't make sense to serialize them to json, or restore them.
    }
}
exports.Reflection = Reflection;
