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
  export function s32(property?: string): Layout<number>;
  export function u32(property?: string): Layout<number>;
  export function s16(property?: string): Layout<number>;
  export function u16(property?: string): Layout<number>;
  export function s8(property?: string): Layout<number>;
  export function u8(property?: string): Layout<number>;
}
