import type { Application } from "../application";
import { EventDispatcher, Event, EventMap } from "./events";
/**
 * Exposes a reference to the root Application component.
 */
export interface ComponentHost {
    readonly application: Application;
}
export interface Component extends AbstractComponent<ComponentHost> {
}
export interface ComponentClass<T extends Component, O extends ComponentHost = ComponentHost> extends Function {
    new (owner: O): T;
}
/**
 * Option-bag passed to Component decorator.
 */
export interface ComponentOptions {
    name?: string;
    /** Specify valid child component class.  Used to prove that children are valid via `instanceof` checks */
    childClass?: Function;
    internal?: boolean;
}
/**
 * Class decorator applied to Components
 */
export declare function Component(options: ComponentOptions): ClassDecorator;
export declare class ComponentEvent extends Event {
    owner: ComponentHost;
    component: AbstractComponent<ComponentHost>;
    static ADDED: string;
    static REMOVED: string;
    constructor(name: string, owner: ComponentHost, component: AbstractComponent<ComponentHost>);
}
/**
 * Component base class.  Has an owner (unless it's the application root component),
 * can dispatch events to its children, and has access to the root Application component.
 *
 * @template O type of component's owner.
 */
export declare abstract class AbstractComponent<O extends ComponentHost> extends EventDispatcher implements ComponentHost {
    /**
     * The owner of this component instance.
     */
    private _componentOwner;
    /**
     * The name of this component as set by the @Component decorator.
     */
    componentName: string;
    /**
     * Create new Component instance.
     */
    constructor(owner: O);
    /**
     * Initialize this component.
     */
    protected initialize(): void;
    protected bubble(name: Event | EventMap | string, ...args: any[]): this;
    /**
     * Return the application / root component instance.
     */
    get application(): Application;
    /**
     * Return the owner of this component.
     */
    get owner(): O;
}
/**
 * Component that can have child components.
 *
 * @template O type of Component's owner
 * @template C type of Component's children
 */
export declare abstract class ChildableComponent<O extends ComponentHost, C extends Component> extends AbstractComponent<O> {
    /**
     *
     */
    private _componentChildren?;
    private _defaultComponents?;
    /**
     * Create new Component instance.
     */
    constructor(owner: O);
    /**
     * Retrieve a plugin instance.
     *
     * @returns  The instance of the plugin or undefined if no plugin with the given class is attached.
     */
    getComponent(name: string): C | undefined;
    getComponents(): C[];
    hasComponent(name: string): boolean;
    addComponent<T extends C>(name: string, componentClass: T | ComponentClass<T, O>): T;
    removeComponent(name: string): C | undefined;
    removeAllComponents(): void;
}
