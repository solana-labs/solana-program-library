import { ChangeLogEventV1 } from "../types";
import { accountCompressionEventBeet } from "../generated/types/AccountCompressionEvent";
import BN from 'bn.js';
import { ApplicationDataEvent, ChangeLogEvent, ChangeLogEventV1 as CLV1 } from "../generated";

export function deserializeChangeLogEventV1(data: Buffer): ChangeLogEventV1 {
    const event = accountCompressionEventBeet.toFixedFromData(data, 0).read(data, 0)
    switch (event.__kind) {
        case "ChangeLog": {
            switch (event.fields[0].__kind) {
                case "V1":
                    const changeLogV1: CLV1 = event.fields[0].fields[0];
                    return {
                        treeId: changeLogV1.id,
                        seq: new BN.BN(changeLogV1.seq),
                        path: changeLogV1.path,
                        index: changeLogV1.index,
                    }
            }
        }
        default:
            throw Error("Unable to decode buffer as ChangeLogEvent V1");
    }
}

export function deserializeApplicationDataEvent(data: Buffer): ApplicationDataEvent {
    const event = accountCompressionEventBeet.toFixedFromData(data, 0).read(data, 0)
    switch (event.__kind) {
        case "ApplicationData": {
            return event.fields[0]
        }
        default:
            throw Error("Unable to decode buffer as ApplicationDataEvent");
    }
}