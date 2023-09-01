import type { RenderTemplate } from "../../..";
import type { Reflection } from "../../../../models";
import { JSX } from "../../../../utils";
import type { PageEvent } from "../../../events";
import type { DefaultThemeRenderContext } from "../DefaultThemeRenderContext";
export declare const defaultLayout: (context: DefaultThemeRenderContext, template: RenderTemplate<PageEvent<Reflection>>, props: PageEvent<Reflection>) => JSX.Element;
