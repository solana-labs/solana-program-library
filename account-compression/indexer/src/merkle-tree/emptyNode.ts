
import { hash } from "./hash";


/**
 * In-memory caching of recursive hashes of empty leaf nodes
 */
const emptyNodeCache = new Map<number, Buffer>();

/**
 * Recursively hashes empty nodes to `level`
 */
export function emptyNode(level: number): Buffer {
    if (emptyNodeCache.has(level)) {
        return emptyNodeCache.get(level);
    }
    if (level == 0) {
        return Buffer.alloc(32);
    }
    let result = hash(emptyNode(level - 1), emptyNode(level - 1));
    emptyNodeCache.set(level, result);
    return result;
}