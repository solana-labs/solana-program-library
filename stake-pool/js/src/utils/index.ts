export * from './math';
export * from './program-address';
export * from './stake';
export * from './instruction';

export function arrayChunk(array: any[], size: number): any[] {
  const result = [];
  for (let i = 0; i < array.length; i += size) {
    result.push(array.slice(i, i + size));
  }
  return result;
}
