declare module 'buffer-layout' {
  export class Layout<T = any> {
    span: number;
    property?: string;
    constructor(span: number, property?: string);
    decode(b: Buffer | undefined, offset?: number): T;
    encode(src: T, b: Buffer, offset?: number): number;
    getSpan(b: Buffer, offset?: number): number;
    replicate(name: string): this;
  }
  export function struct<T>(
    fields: Layout<any>[],
    property?: string,
    decodePrefixes?: boolean,
  ): Layout<T>;
  export function seq<T>(
    elementLayout: Layout<T>,
    count: number | Layout<number>,
    property?: string,
  ): Layout<T[]>;
  export function offset<T>(layout: Layout<T>, offset?: number, property?: string): Layout<T>;
  export function blob(length: number | Layout<number>, property?: string): Layout<Buffer>;
  export function s32(property?: string): Layout<number>;
  export function u32(property?: string): Layout<number>;
  export function s16(property?: string): Layout<number>;
  export function u16(property?: string): Layout<number>;
  export function s8(property?: string): Layout<number>;
  export function u8(property?: string): Layout<number>;
}
