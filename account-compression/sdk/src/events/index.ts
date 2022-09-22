import { ChangeLogEventV1 } from "../types";
import { accountCompressionEventBeet } from "../generated/types/AccountCompressionEvent";
import BN from 'bn.js';

export type AccountCompressionEventType = ChangeLogEventV1;

export function deserializeAccountCompressionEvent(data: Buffer): AccountCompressionEventType {
    const event = accountCompressionEventBeet.toFixedFromData(data, 0).read(data, 0)
    switch (event.__kind) {
        case "ChangeLog": {
            switch (event.fields[0].__kind) {
                case "V1":
                    const changeLogV1 = event.fields[0].fields[0];
                    return {
                        treeId: changeLogV1.id,
                        seq: new BN.BN(changeLogV1.seq),
                        path: changeLogV1.path,
                        index: changeLogV1.index,
                    }
            }
        }
    }
}