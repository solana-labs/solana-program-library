"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ReflectionGroup = void 0;
const ReflectionCategory_1 = require("./ReflectionCategory");
/**
 * A group of reflections. All reflections in a group are of the same kind.
 *
 * Reflection groups are created by the ´GroupHandler´ in the resolving phase
 * of the dispatcher. The main purpose of groups is to be able to more easily
 * render human readable children lists in templates.
 */
class ReflectionGroup {
    /**
     * Create a new ReflectionGroup instance.
     *
     * @param title The title of this group.
     */
    constructor(title) {
        /**
         * All reflections of this group.
         */
        this.children = [];
        this.title = title;
    }
    /**
     * Do all children of this group have a separate document?
     */
    allChildrenHaveOwnDocument() {
        return this.children.every((child) => child.hasOwnDocument);
    }
    toObject(serializer) {
        return {
            title: this.title,
            children: this.children.length > 0
                ? this.children.map((child) => child.id)
                : undefined,
            categories: serializer.toObjectsOptional(this.categories),
        };
    }
    fromObject(de, obj) {
        if (obj.categories) {
            this.categories = obj.categories.map((catObj) => {
                const cat = new ReflectionCategory_1.ReflectionCategory(catObj.title);
                de.fromObject(cat, catObj);
                return cat;
            });
        }
        if (obj.children) {
            de.defer((project) => {
                for (const childId of obj.children || []) {
                    const child = project.getReflectionById(de.oldIdToNewId[childId] ?? -1);
                    if (child?.isDeclaration()) {
                        this.children.push(child);
                    }
                }
            });
        }
    }
}
exports.ReflectionGroup = ReflectionGroup;
