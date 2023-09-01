/**
 * Module which handles sorting reflections according to a user specified strategy.
 * @module
 */
import type { DeclarationReflection } from "../models/reflections/declaration";
import type { Options } from "./options";
export declare const SORT_STRATEGIES: readonly ["source-order", "alphabetical", "enum-value-ascending", "enum-value-descending", "enum-member-source-order", "static-first", "instance-first", "visibility", "required-first", "kind"];
export type SortStrategy = (typeof SORT_STRATEGIES)[number];
export declare function getSortFunction(opts: Options): (reflections: DeclarationReflection[]) => void;
