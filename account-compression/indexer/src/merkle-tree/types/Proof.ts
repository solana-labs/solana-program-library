export type Proof = {
    /** Root that this proof is valid for */
    root: string;
    /** Merkle proof nodes */
    proofNodes: string[];
    /** Leaf-index */
    index: number;
    /** Current leaf value at index */
    leaf: string;
};