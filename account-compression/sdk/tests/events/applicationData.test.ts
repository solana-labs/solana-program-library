import { BN } from 'bn.js';
import { assert } from 'chai';

import { deserializeApplicationDataEvent } from '../../src';

describe('Serde tests', () => {
  describe('ApplicationDataEvent tests', () => {
    it('Can serialize and deserialize ApplicationDataEvent', async () => {
      const data = Buffer.from('Hello world');
      const applicationDataEvent = Buffer.concat([
        Buffer.from([0x1]), // ApplicationData Event tag
        Buffer.from([0x0]), // version 0 tag
        Buffer.from(new BN.BN(data.length).toArray('le', 4)), // Size of application data (for Vec)
        data, // serialized application data (for Vec)
      ]);

      const deserialized =
        deserializeApplicationDataEvent(applicationDataEvent);
      const decoder = new TextDecoder();
      const deserializedData = decoder.decode(
        deserialized.fields[0].applicationData
      );
      assert('Hello world' === deserializedData);
    });
  });
});
