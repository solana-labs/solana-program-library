/// Convert a 32 bit number to a buffer of bytes
export function num32ToBuffer(num: number) {
  const isU32 = (num >= 0 && num < Math.pow(2,32));
  const isI32 = (num >= -1*Math.pow(2, 31) && num < Math.pow(2,31))
  if (!isU32 || !isI32) {
    throw new Error("Attempted to convert non 32 bit integer to byte array")
  }
  var byte1 = 0xff & num;
  var byte2 = 0xff & (num >> 8);
  var byte3 = 0xff & (num >> 16);
  var byte4 = 0xff & (num >> 24);
  return Buffer.from([byte1, byte2, byte3, byte4])
}

/// Check if two Array types contain the same values in order
export function arrayEquals(a, b) {
  return Array.isArray(a) &&
      Array.isArray(b) &&
      a.length === b.length &&
      a.every((val, index) => val === b[index]);
}

/// Convert Buffer to Uint8Array
export function bufferToArray(buffer: Buffer): number[] {
  const nums = [];
  for (let i = 0; i < buffer.length; i++) {
    nums.push(buffer.at(i));
  }
  return nums;
}
