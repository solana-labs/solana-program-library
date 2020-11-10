declare module 'buffer-layout' {
  export type Layout = {
    encode(src: unknown, b: Buffer, number?: offset): number;
    decode(b: Buffer, number?: offset): number | array | object;
    span: number;
  };

  export function struct(fields: Layout[], property?: string): Layout;
  export function blob(value: number, property: string): Layout;

  export function u8(property?: string): Layout;
  export function u32(property?: string): Layout;

  export function nu64(property?: string): Layout;

  export function offset(
    layout: Layout,
    offset: number,
    property?: string
  ): number;
}
