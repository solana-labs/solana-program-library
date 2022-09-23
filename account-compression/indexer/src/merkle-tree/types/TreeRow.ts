export type TreeRow = {
    /** 
     * Index of the node in the tree, performed by a breadth first traversal 
     * from the root to the leaves (based on the maxHeight of the tree) 
     */
    nodeIndex: number,
    /** Bytes of the node */
    hash: string,
    /** Height of the node in the tree */
    level: number,
    /** Sequence number */
    seq: number
}