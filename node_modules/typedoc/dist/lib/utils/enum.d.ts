export declare function getEnumFlags<T extends number>(flags: T): T[];
export declare function removeFlag<T extends number>(flag: T, remove: T & {}): T;
export declare function hasAllFlags(flags: number, check: number): boolean;
export declare function hasAnyFlag(flags: number, check: number): boolean;
export declare function getEnumKeys(Enum: {}): string[];
export type EnumKeys<E extends {}> = keyof {
    [K in keyof E as number extends E[K] ? K : never]: 1;
};
