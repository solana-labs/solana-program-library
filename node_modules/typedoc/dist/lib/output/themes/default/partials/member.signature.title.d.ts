import type { DefaultThemeRenderContext } from "../DefaultThemeRenderContext";
import { JSX } from "../../../../utils";
import { SignatureReflection } from "../../../../models";
export declare function memberSignatureTitle(context: DefaultThemeRenderContext, props: SignatureReflection, { hideName, arrowStyle }?: {
    hideName?: boolean;
    arrowStyle?: boolean;
}): JSX.Element;
