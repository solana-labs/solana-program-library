
export function toNodeIndex(leafIndex: number, maxDepth: number): number {
    return (1 << maxDepth) + leafIndex;
}

export function toLeafIndex(nodeIndex: number, maxDepth: number): number {
    return nodeIndex - (1 << maxDepth);
}