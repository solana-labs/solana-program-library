import type { PageEvent, RendererHooks } from "../..";
import { CommentDisplayPart, DeclarationReflection, ReferenceType, Reflection } from "../../../models";
import type { NeverIfInternal, Options } from "../../../utils";
import type { DefaultTheme } from "./DefaultTheme";
export declare class DefaultThemeRenderContext {
    private theme;
    page: PageEvent<Reflection>;
    options: Options;
    constructor(theme: DefaultTheme, page: PageEvent<Reflection>, options: Options);
    icons: Record<"search" | "anchor" | import("../../../models").ReflectionKind | "checkbox" | "chevronDown" | "menu" | "chevronSmall", () => import("../../../utils/jsx.elements").JsxElement>;
    hook: (name: keyof RendererHooks) => import("../../../utils/jsx.elements").JsxElement[];
    /** Avoid this in favor of urlTo if possible */
    relativeURL: (url: string, cacheBust?: boolean) => string;
    urlTo: (reflection: Reflection) => string;
    markdown: (md: readonly CommentDisplayPart[] | NeverIfInternal<string | undefined>) => string;
    /**
     * Using this method will repeat work already done, instead of calling it, use `type.externalUrl`.
     * @deprecated
     * Will be removed in 0.24.
     */
    attemptExternalResolution: (type: NeverIfInternal<ReferenceType>) => string | undefined;
    getReflectionClasses: (refl: DeclarationReflection) => string;
    reflectionTemplate: (props: PageEvent<import("../../../models").ContainerReflection>) => import("../../../utils/jsx.elements").JsxElement;
    indexTemplate: (props: PageEvent<import("../../../models").ProjectReflection>) => import("../../../utils/jsx.elements").JsxElement;
    defaultLayout: (template: import("../..").RenderTemplate<PageEvent<Reflection>>, props: PageEvent<Reflection>) => import("../../../utils/jsx.elements").JsxElement;
    analytics: () => import("../../../utils/jsx.elements").JsxElement | undefined;
    breadcrumb: (props: Reflection) => import("../../../utils/jsx.elements").JsxElement | undefined;
    comment: (props: Reflection) => import("../../../utils/jsx.elements").JsxElement | undefined;
    footer: () => import("../../../utils/jsx.elements").JsxElement | undefined;
    header: (props: PageEvent<Reflection>) => import("../../../utils/jsx.elements").JsxElement;
    hierarchy: (props: import("../../../models").DeclarationHierarchy | undefined) => import("../../../utils/jsx.elements").JsxElement | undefined;
    index: (props: import("../../../models").ContainerReflection) => import("../../../utils/jsx.elements").JsxElement;
    member: (props: DeclarationReflection) => import("../../../utils/jsx.elements").JsxElement;
    memberDeclaration: (props: DeclarationReflection) => import("../../../utils/jsx.elements").JsxElement;
    memberGetterSetter: (props: DeclarationReflection) => import("../../../utils/jsx.elements").JsxElement;
    memberReference: (props: import("../../../models").ReferenceReflection) => import("../../../utils/jsx.elements").JsxElement;
    memberSignatureBody: (r_0: import("../../../models").SignatureReflection, r_1?: {
        hideSources?: boolean | undefined;
    } | undefined) => import("../../../utils/jsx.elements").JsxElement;
    memberSignatureTitle: (r_0: import("../../../models").SignatureReflection, r_1?: {
        hideName?: boolean | undefined;
        arrowStyle?: boolean | undefined;
    } | undefined) => import("../../../utils/jsx.elements").JsxElement;
    memberSignatures: (props: DeclarationReflection) => import("../../../utils/jsx.elements").JsxElement;
    memberSources: (props: import("../../../models").SignatureReflection | DeclarationReflection) => import("../../../utils/jsx.elements").JsxElement;
    members: (props: import("../../../models").ContainerReflection) => import("../../../utils/jsx.elements").JsxElement;
    membersGroup: (group: import("../../../models").ReflectionGroup) => import("../../../utils/jsx.elements").JsxElement;
    sidebar: (props: PageEvent<Reflection>) => import("../../../utils/jsx.elements").JsxElement;
    pageSidebar: (props: PageEvent<Reflection>) => import("../../../utils/jsx.elements").JsxElement;
    sidebarLinks: () => import("../../../utils/jsx.elements").JsxElement | null;
    settings: () => import("../../../utils/jsx.elements").JsxElement;
    navigation: (props: PageEvent<Reflection>) => import("../../../utils/jsx.elements").JsxElement;
    pageNavigation: (props: PageEvent<Reflection>) => import("../../../utils/jsx.elements").JsxElement;
    parameter: (props: DeclarationReflection) => import("../../../utils/jsx.elements").JsxElement;
    toolbar: (props: PageEvent<Reflection>) => import("../../../utils/jsx.elements").JsxElement;
    type: (type: import("../../../models").Type | undefined) => import("../../../utils/jsx.elements").JsxElement;
    typeAndParent: (props: import("../../../models").Type) => import("../../../utils/jsx.elements").JsxElement;
    typeParameters: (typeParameters: import("../../../models").TypeParameterReflection[]) => import("../../../utils/jsx.elements").JsxElement;
}
