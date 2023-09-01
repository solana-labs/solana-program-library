"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
var CategoryPlugin_1;
Object.defineProperty(exports, "__esModule", { value: true });
exports.CategoryPlugin = void 0;
const models_1 = require("../../models");
const models_2 = require("../../models");
const components_1 = require("../components");
const converter_1 = require("../converter");
const utils_1 = require("../../utils");
/**
 * A handler that sorts and categorizes the found reflections in the resolving phase.
 *
 * The handler sets the ´category´ property of all reflections.
 */
let CategoryPlugin = CategoryPlugin_1 = class CategoryPlugin extends components_1.ConverterComponent {
    constructor() {
        super(...arguments);
        this.usedBoosts = new Set();
    }
    /**
     * Create a new CategoryPlugin instance.
     */
    initialize() {
        this.listenTo(this.owner, {
            [converter_1.Converter.EVENT_BEGIN]: this.onBegin,
            [converter_1.Converter.EVENT_RESOLVE]: this.onResolve,
            [converter_1.Converter.EVENT_RESOLVE_END]: this.onEndResolve,
        }, undefined, -200);
    }
    /**
     * Triggered when the converter begins converting a project.
     */
    onBegin(_context) {
        this.sortFunction = (0, utils_1.getSortFunction)(this.application.options);
        // Set up static properties
        if (this.defaultCategory) {
            CategoryPlugin_1.defaultCategory = this.defaultCategory;
        }
        if (this.categoryOrder) {
            CategoryPlugin_1.WEIGHTS = this.categoryOrder;
        }
    }
    /**
     * Triggered when the converter resolves a reflection.
     *
     * @param context  The context object describing the current state the converter is in.
     * @param reflection  The reflection that is currently resolved.
     */
    onResolve(_context, reflection) {
        if (reflection instanceof models_1.ContainerReflection) {
            this.categorize(reflection);
        }
    }
    /**
     * Triggered when the converter has finished resolving a project.
     *
     * @param context  The context object describing the current state the converter is in.
     */
    onEndResolve(context) {
        const project = context.project;
        this.categorize(project);
        const unusedBoosts = new Set(Object.keys(this.boosts));
        for (const boost of this.usedBoosts) {
            unusedBoosts.delete(boost);
        }
        this.usedBoosts.clear();
        if (unusedBoosts.size) {
            context.logger.warn(`Not all categories specified in searchCategoryBoosts were used in the documentation.` +
                ` The unused categories were:\n\t${Array.from(unusedBoosts).join("\n\t")}`);
        }
    }
    categorize(obj) {
        if (this.categorizeByGroup) {
            this.groupCategorize(obj);
        }
        else {
            this.lumpCategorize(obj);
        }
    }
    groupCategorize(obj) {
        if (!obj.groups || obj.groups.length === 0) {
            return;
        }
        obj.groups.forEach((group) => {
            if (group.categories)
                return;
            group.categories = this.getReflectionCategories(group.children);
            if (group.categories && group.categories.length > 1) {
                group.categories.sort(CategoryPlugin_1.sortCatCallback);
            }
            else if (group.categories.length === 1 &&
                group.categories[0].title === CategoryPlugin_1.defaultCategory) {
                // no categories if everything is uncategorized
                group.categories = undefined;
            }
        });
    }
    lumpCategorize(obj) {
        if (!obj.children || obj.children.length === 0 || obj.categories) {
            return;
        }
        obj.categories = this.getReflectionCategories(obj.children);
        if (obj.categories && obj.categories.length > 1) {
            obj.categories.sort(CategoryPlugin_1.sortCatCallback);
        }
        else if (obj.categories.length === 1 &&
            obj.categories[0].title === CategoryPlugin_1.defaultCategory) {
            // no categories if everything is uncategorized
            obj.categories = undefined;
        }
    }
    /**
     * Create a categorized representation of the given list of reflections.
     *
     * @param reflections  The reflections that should be categorized.
     * @param categorySearchBoosts A user-supplied map of category titles, for computing a
     *   relevance boost to be used when searching
     * @returns An array containing all children of the given reflection categorized
     */
    getReflectionCategories(reflections) {
        const categories = new Map();
        for (const child of reflections) {
            const childCategories = this.getCategories(child);
            if (childCategories.size === 0) {
                childCategories.add(CategoryPlugin_1.defaultCategory);
            }
            for (const childCat of childCategories) {
                const category = categories.get(childCat);
                if (category) {
                    category.children.push(child);
                }
                else {
                    const cat = new models_2.ReflectionCategory(childCat);
                    cat.children.push(child);
                    categories.set(childCat, cat);
                }
            }
        }
        for (const cat of categories.values()) {
            this.sortFunction(cat.children);
        }
        return Array.from(categories.values());
    }
    /**
     * Return the category of a given reflection.
     *
     * @param reflection The reflection.
     * @returns The category the reflection belongs to
     *
     * @privateRemarks
     * If you change this, also update getGroups in GroupPlugin accordingly.
     */
    getCategories(reflection) {
        const categories = new Set();
        function extractCategoryTags(comment) {
            if (!comment)
                return;
            (0, utils_1.removeIf)(comment.blockTags, (tag) => {
                if (tag.tag === "@category") {
                    categories.add(models_1.Comment.combineDisplayParts(tag.content).trim());
                    return true;
                }
                return false;
            });
        }
        extractCategoryTags(reflection.comment);
        for (const sig of reflection.getNonIndexSignatures()) {
            extractCategoryTags(sig.comment);
        }
        if (reflection.type?.type === "reflection") {
            extractCategoryTags(reflection.type.declaration.comment);
            for (const sig of reflection.type.declaration.getNonIndexSignatures()) {
                extractCategoryTags(sig.comment);
            }
        }
        categories.delete("");
        for (const cat of categories) {
            if (cat in this.boosts) {
                this.usedBoosts.add(cat);
                reflection.relevanceBoost =
                    (reflection.relevanceBoost ?? 1) * this.boosts[cat];
            }
        }
        return categories;
    }
    /**
     * Callback used to sort categories by name.
     *
     * @param a The left reflection to sort.
     * @param b The right reflection to sort.
     * @returns The sorting weight.
     */
    static sortCatCallback(a, b) {
        let aWeight = CategoryPlugin_1.WEIGHTS.indexOf(a.title);
        let bWeight = CategoryPlugin_1.WEIGHTS.indexOf(b.title);
        if (aWeight === -1 || bWeight === -1) {
            let asteriskIndex = CategoryPlugin_1.WEIGHTS.indexOf("*");
            if (asteriskIndex === -1) {
                asteriskIndex = CategoryPlugin_1.WEIGHTS.length;
            }
            if (aWeight === -1) {
                aWeight = asteriskIndex;
            }
            if (bWeight === -1) {
                bWeight = asteriskIndex;
            }
        }
        if (aWeight === bWeight) {
            return a.title > b.title ? 1 : -1;
        }
        return aWeight - bWeight;
    }
};
// For use in static methods
CategoryPlugin.defaultCategory = "Other";
CategoryPlugin.WEIGHTS = [];
__decorate([
    (0, utils_1.BindOption)("defaultCategory")
], CategoryPlugin.prototype, "defaultCategory", void 0);
__decorate([
    (0, utils_1.BindOption)("categoryOrder")
], CategoryPlugin.prototype, "categoryOrder", void 0);
__decorate([
    (0, utils_1.BindOption)("categorizeByGroup")
], CategoryPlugin.prototype, "categorizeByGroup", void 0);
__decorate([
    (0, utils_1.BindOption)("searchCategoryBoosts")
], CategoryPlugin.prototype, "boosts", void 0);
CategoryPlugin = CategoryPlugin_1 = __decorate([
    (0, components_1.Component)({ name: "category" })
], CategoryPlugin);
exports.CategoryPlugin = CategoryPlugin;
