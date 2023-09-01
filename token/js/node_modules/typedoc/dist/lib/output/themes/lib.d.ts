import type { DefaultThemeRenderContext } from "..";
import { Comment, Reflection, ReflectionFlags, TypeParameterReflection } from "../../models";
import { JSX } from "../../utils";
export declare function stringify(data: unknown): string;
export declare function getDisplayName(refl: Reflection): string;
export declare function toStyleClass(str: string): string;
export declare function getKindClass(refl: Reflection): string;
/**
 * Insert word break tags ``<wbr>`` into the given string.
 *
 * Breaks the given string at ``_``, ``-`` and capital letters.
 *
 * @param str The string that should be split.
 * @return The original string containing ``<wbr>`` tags where possible.
 */
export declare function wbr(str: string): (string | JSX.Element)[];
export declare function join<T>(joiner: JSX.Children, list: readonly T[], cb: (x: T) => JSX.Children): JSX.Element;
export declare function renderFlags(flags: ReflectionFlags, comment: Comment | undefined): JSX.Element;
export declare function classNames(names: Record<string, boolean | null | undefined>, extraCss?: string): string | undefined;
export declare function hasTypeParameters(reflection: Reflection): reflection is Reflection & {
    typeParameters: TypeParameterReflection[];
};
export declare function renderTypeParametersSignature(context: DefaultThemeRenderContext, typeParameters: readonly TypeParameterReflection[] | undefined): JSX.Element;
export declare function camelToTitleCase(text: string): string;
/**
 * Renders the reflection name with an additional `?` if optional.
 */
export declare function renderName(refl: Reflection): JSX.Element | (string | JSX.Element)[];
