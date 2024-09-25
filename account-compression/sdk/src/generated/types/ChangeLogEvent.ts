/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet';

import { ChangeLogEventV1, changeLogEventV1Beet } from './ChangeLogEventV1';
/**
 * This type is used to derive the {@link ChangeLogEvent} type as well as the de/serializer.
 * However don't refer to it in your code but use the {@link ChangeLogEvent} type instead.
 *
 * @category userTypes
 * @category enums
 * @category generated
 * @private
 */
export type ChangeLogEventRecord = {
    V1: { fields: [ChangeLogEventV1] };
};

/**
 * Union type respresenting the ChangeLogEvent data enum defined in Rust.
 *
 * NOTE: that it includes a `__kind` property which allows to narrow types in
 * switch/if statements.
 * Additionally `isChangeLogEvent*` type guards are exposed below to narrow to a specific variant.
 *
 * @category userTypes
 * @category enums
 * @category generated
 */
export type ChangeLogEvent = beet.DataEnumKeyAsKind<ChangeLogEventRecord>;

export const isChangeLogEventV1 = (x: ChangeLogEvent): x is ChangeLogEvent & { __kind: 'V1' } => x.__kind === 'V1';

/**
 * @category userTypes
 * @category generated
 */
export const changeLogEventBeet = beet.dataEnum<ChangeLogEventRecord>([
    [
        'V1',
        new beet.FixableBeetArgsStruct<ChangeLogEventRecord['V1']>(
            [['fields', beet.tuple([changeLogEventV1Beet])]],
            'ChangeLogEventRecord["V1"]',
        ),
    ],
]) as beet.FixableBeet<ChangeLogEvent, ChangeLogEvent>;
