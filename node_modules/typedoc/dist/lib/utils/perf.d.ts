export declare function bench<T extends Function>(fn: T, name?: string): T;
export declare function Bench(): MethodDecorator;
export declare function measure<T>(cb: () => T): T;
