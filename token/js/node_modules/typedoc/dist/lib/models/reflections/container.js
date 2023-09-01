"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ContainerReflection = void 0;
const abstract_1 = require("./abstract");
const ReflectionCategory_1 = require("../ReflectionCategory");
const ReflectionGroup_1 = require("../ReflectionGroup");
class ContainerReflection extends abstract_1.Reflection {
    /**
     * Return a list of all children of a certain kind.
     *
     * @param kind  The desired kind of children.
     * @returns     An array containing all children with the desired kind.
     */
    getChildrenByKind(kind) {
        return (this.children || []).filter((child) => child.kindOf(kind));
    }
    traverse(callback) {
        for (const child of this.children?.slice() || []) {
            if (callback(child, abstract_1.TraverseProperty.Children) === false) {
                return;
            }
        }
    }
    toObject(serializer) {
        return {
            ...super.toObject(serializer),
            children: serializer.toObjectsOptional(this.children),
            groups: serializer.toObjectsOptional(this.groups),
            categories: serializer.toObjectsOptional(this.categories),
        };
    }
    fromObject(de, obj) {
        super.fromObject(de, obj);
        this.children = de.reviveMany(obj.children, (child) => de.constructReflection(child));
        this.groups = de.reviveMany(obj.groups, (group) => new ReflectionGroup_1.ReflectionGroup(group.title));
        this.categories = de.reviveMany(obj.categories, (cat) => new ReflectionCategory_1.ReflectionCategory(cat.title));
    }
}
exports.ContainerReflection = ContainerReflection;
