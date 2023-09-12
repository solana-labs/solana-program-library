# Type-Length-Value-js

Library with utilities for working with Type-Length-Value structures in js.

## Example usage

```ts
import { TlvState, SplDiscriminator } from '@solana/spl-type-length-value';

const tlv = new TlvState(tlvData, typeSize, lengthSize);
const discriminator = new SplDiscriminator("<discriminator-key>");

const firstValue = tlv.firstBytes(discriminator);

const allValues = tlv.bytesRepeating(discriminator);

const firstThreeValues = tlv.bytesRepeating(discriminator, 3);
```
