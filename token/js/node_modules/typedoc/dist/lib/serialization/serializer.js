"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Serializer = void 0;
const utils_1 = require("../utils");
const events_1 = require("./events");
const array_1 = require("../utils/array");
class Serializer extends utils_1.EventDispatcher {
    constructor() {
        super(...arguments);
        this.serializers = [];
    }
    addSerializer(serializer) {
        (0, array_1.insertPrioritySorted)(this.serializers, serializer);
    }
    toObject(value) {
        if (value === undefined) {
            return undefined;
        }
        return this.serializers
            .filter((s) => s.supports(value))
            .reduce((val, s) => s.toObject(value, val, this), value.toObject(this));
    }
    toObjectsOptional(value) {
        if (!value || value.length === 0) {
            return undefined;
        }
        return value.map((val) => this.toObject(val));
    }
    /**
     * Same as toObject but emits {@link Serializer.EVENT_BEGIN} and {@link Serializer.EVENT_END} events.
     * @param value
     * @param eventData Partial information to set in the event
     */
    projectToObject(value, projectRoot) {
        this.projectRoot = projectRoot;
        const eventBegin = new events_1.SerializeEvent(Serializer.EVENT_BEGIN, value);
        this.trigger(eventBegin);
        const project = this.toObject(value);
        const eventEnd = new events_1.SerializeEvent(Serializer.EVENT_END, value, project);
        this.trigger(eventEnd);
        return project;
    }
}
/**
 * Triggered when the {@link Serializer} begins transforming a project.
 * @event EVENT_BEGIN
 */
Serializer.EVENT_BEGIN = "begin";
/**
 * Triggered when the {@link Serializer} has finished transforming a project.
 * @event EVENT_END
 */
Serializer.EVENT_END = "end";
exports.Serializer = Serializer;
