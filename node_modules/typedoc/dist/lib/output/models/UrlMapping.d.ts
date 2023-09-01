import type { JSX } from "../../utils";
import type { PageEvent } from "../events";
export declare class UrlMapping<Model = any> {
    url: string;
    model: Model;
    template: RenderTemplate<PageEvent<Model>>;
    constructor(url: string, model: Model, template: RenderTemplate<PageEvent<Model>>);
}
export type RenderTemplate<T> = (data: T) => JSX.Element | string;
