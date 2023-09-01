export interface EventCallback extends Function {
    _callback?: Function;
}
export interface EventMap {
    [name: string]: EventCallback;
}
/**
 * An event object.
 */
export declare class Event {
    /**
     * The name of the event.
     */
    private _name;
    /**
     * Has {@link Event.stopPropagation} been called?
     */
    private _isPropagationStopped;
    /**
     * Has {@link Event.preventDefault} been called?
     */
    private _isDefaultPrevented;
    /**
     * Create a new Event instance.
     */
    constructor(name: string);
    /**
     * Stop the propagation of this event. Remaining event handlers will not be executed.
     */
    stopPropagation(): void;
    /**
     * Prevent the default action associated with this event from being executed.
     */
    preventDefault(): void;
    /**
     * Return the event name.
     */
    get name(): string;
    /**
     * Has {@link Event.stopPropagation} been called?
     */
    get isPropagationStopped(): boolean;
    /**
     * Has {@link Event.preventDefault} been called?
     */
    get isDefaultPrevented(): boolean;
}
/**
 * A class that provides a custom event channel.
 *
 * You may bind a callback to an event with `on` or remove with `off`;
 * `trigger`-ing an event fires all callbacks in succession.
 */
export declare class EventDispatcher {
    /**
     * Map of all handlers registered with the "on" function.
     */
    private _events?;
    /**
     * Map of all objects this instance is listening to.
     */
    private _listeningTo?;
    /**
     * Map of all objects that are listening to this instance.
     */
    private _listeners?;
    /**
     * A unique id that identifies this instance.
     */
    private get _listenId();
    private _savedListenId?;
    /**
     * Bind an event to a `callback` function. Passing `"all"` will bind
     * the callback to all events fired.
     */
    on(eventMap: EventMap, context?: any): this;
    on(eventMap: EventMap, callback?: EventCallback, context?: any, priority?: number): this;
    on(name: string, callback: EventCallback, context?: any, priority?: number): this;
    /**
     * Guard the `listening` argument from the public API.
     */
    private internalOn;
    /**
     * Bind an event to only be triggered a single time. After the first time
     * the callback is invoked, its listener will be removed. If multiple events
     * are passed in using the space-separated syntax, the handler will fire
     * once for each event, not once for a combination of all events.
     */
    once(eventMap: EventMap, context?: any): this;
    once(name: string, callback: EventCallback, context?: any, priority?: any): this;
    /**
     * Remove one or many callbacks. If `context` is null, removes all
     * callbacks with that function. If `callback` is null, removes all
     * callbacks for the event. If `name` is null, removes all bound
     * callbacks for all events.
     */
    off(): this;
    off(eventMap: EventMap | undefined, context?: any): this;
    off(name: string | undefined, callback?: EventCallback, context?: any): this;
    /**
     * Inversion-of-control versions of `on`. Tell *this* object to listen to
     * an event in another object... keeping track of what it's listening to
     * for easier unbinding later.
     */
    listenTo(obj: EventDispatcher, name: EventMap | string, callback?: EventCallback, priority?: number): this;
    /**
     * Inversion-of-control versions of `once`.
     */
    listenToOnce(obj: EventDispatcher, eventMap: EventMap): this;
    listenToOnce(obj: EventDispatcher, name: string, callback: EventCallback, priority?: number): this;
    /**
     * Tell this object to stop listening to either specific events ... or
     * to every object it's currently listening to.
     */
    stopListening(obj?: EventDispatcher, name?: EventMap | string, callback?: EventCallback): this;
    /**
     * Trigger one or many events, firing all bound callbacks. Callbacks are
     * passed the same arguments as `trigger` is, apart from the event name
     * (unless you're listening on `"all"`, which will cause your callback to
     * receive the true name of the event as the first argument).
     */
    trigger(name: Event | EventMap | string, ...args: any[]): this;
}
