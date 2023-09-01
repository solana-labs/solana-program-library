"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ChildableComponent = exports.AbstractComponent = exports.ComponentEvent = exports.Component = void 0;
const events_1 = require("./events");
const childMappings = [];
/**
 * Class decorator applied to Components
 */
function Component(options) {
    return (target) => {
        const proto = target.prototype;
        if (!(proto instanceof AbstractComponent)) {
            throw new Error("The `Component` decorator can only be used with a subclass of `AbstractComponent`.");
        }
        if (options.childClass) {
            if (!(proto instanceof ChildableComponent)) {
                throw new Error("The `Component` decorator accepts the parameter `childClass` only when used with a subclass of `ChildableComponent`.");
            }
            childMappings.push({
                host: proto,
                child: options.childClass,
            });
        }
        const name = options.name;
        if (name) {
            proto.componentName = name;
        }
        // If not marked internal, and if we are a subclass of another component T's declared
        // childClass, then register ourselves as a _defaultComponents of T.
        const internal = !!options.internal;
        if (name && !internal) {
            for (const childMapping of childMappings) {
                if (!(proto instanceof childMapping.child)) {
                    continue;
                }
                const host = childMapping.host;
                host["_defaultComponents"] = host["_defaultComponents"] || {};
                host["_defaultComponents"][name] = target;
                break;
            }
        }
    };
}
exports.Component = Component;
class ComponentEvent extends events_1.Event {
    constructor(name, owner, component) {
        super(name);
        this.owner = owner;
        this.component = component;
    }
}
ComponentEvent.ADDED = "componentAdded";
ComponentEvent.REMOVED = "componentRemoved";
exports.ComponentEvent = ComponentEvent;
/**
 * Component base class.  Has an owner (unless it's the application root component),
 * can dispatch events to its children, and has access to the root Application component.
 *
 * @template O type of component's owner.
 */
class AbstractComponent extends events_1.EventDispatcher {
    /**
     * Create new Component instance.
     */
    constructor(owner) {
        super();
        this._componentOwner = owner;
        this.initialize();
    }
    /**
     * Initialize this component.
     */
    initialize() {
        // empty default implementation
    }
    bubble(name, ...args) {
        super.trigger(name, ...args);
        if (this.owner instanceof AbstractComponent &&
            this._componentOwner !== null) {
            this.owner.bubble(name, ...args);
        }
        return this;
    }
    /**
     * Return the application / root component instance.
     */
    get application() {
        if (this._componentOwner === null) {
            return this;
        }
        return this._componentOwner.application;
    }
    /**
     * Return the owner of this component.
     */
    get owner() {
        return this._componentOwner === null
            ? this
            : this._componentOwner;
    }
}
exports.AbstractComponent = AbstractComponent;
/**
 * Component that can have child components.
 *
 * @template O type of Component's owner
 * @template C type of Component's children
 */
class ChildableComponent extends AbstractComponent {
    /**
     * Create new Component instance.
     */
    constructor(owner) {
        super(owner);
        Object.entries(this._defaultComponents || {}).forEach(([name, component]) => {
            this.addComponent(name, component);
        });
    }
    /**
     * Retrieve a plugin instance.
     *
     * @returns  The instance of the plugin or undefined if no plugin with the given class is attached.
     */
    getComponent(name) {
        return (this._componentChildren || {})[name];
    }
    getComponents() {
        return Object.values(this._componentChildren || {});
    }
    hasComponent(name) {
        return !!(this._componentChildren || {})[name];
    }
    addComponent(name, componentClass) {
        if (!this._componentChildren) {
            this._componentChildren = {};
        }
        if (this._componentChildren[name]) {
            // Component already added so we will return the existing component
            // TODO: add better logging around this because it could be unexpected but shouldn't be fatal
            // See https://github.com/TypeStrong/typedoc/issues/846
            return this._componentChildren[name];
        }
        else {
            const component = typeof componentClass === "function"
                ? new componentClass(this)
                : componentClass;
            const event = new ComponentEvent(ComponentEvent.ADDED, this, component);
            this.bubble(event);
            this._componentChildren[name] = component;
            return component;
        }
    }
    removeComponent(name) {
        const component = (this._componentChildren || {})[name];
        if (component) {
            delete this._componentChildren[name];
            component.stopListening();
            this.bubble(new ComponentEvent(ComponentEvent.REMOVED, this, component));
            return component;
        }
    }
    removeAllComponents() {
        for (const component of Object.values(this._componentChildren || {})) {
            component.stopListening();
        }
        this._componentChildren = {};
    }
}
exports.ChildableComponent = ChildableComponent;
