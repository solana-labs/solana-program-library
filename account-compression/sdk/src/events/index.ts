import BN from 'bn.js';

import { ApplicationDataEvent, ChangeLogEventV1 as CLV1 } from '../generated';
import { accountCompressionEventBeet } from '../generated/types/AccountCompressionEvent';
import { ChangeLogEventV1 } from '../types';

/**
 * Helper method for indexing a {@link ConcurrentMerkleTree}
 * @param data
 * @returns
 */
export function deserializeChangeLogEventV1(data: Buffer): ChangeLogEventV1 {
    const event = accountCompressionEventBeet.toFixedFromData(data, 0).read(data, 0);

    if (event.__kind == 'ChangeLog' && event.fields[0].__kind == 'V1') {
        const changeLogV1: CLV1 = event.fields[0].fields[0];
        return {
            index: changeLogV1.index,
            path: changeLogV1.path,
            seq: new BN.BN(changeLogV1.seq),
            treeId: changeLogV1.id,
        };
    } else {
        throw Error('Unable to decode buffer as ChangeLogEvent V1');
    }
}

/**
 * Helper function for indexing data logged via `wrap_application_data_v1`
 * @param data
 * @returns
 */
export function deserializeApplicationDataEvent(data: Buffer): ApplicationDataEvent {
    const event = accountCompressionEventBeet.toFixedFromData(data, 0).read(data, 0);
    switch (event.__kind) {
        case 'ApplicationData': {
            return event.fields[0];
        }
        default:
            throw Error('Unable to decode buffer as ApplicationDataEvent');
    }
}
