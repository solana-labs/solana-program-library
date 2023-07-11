import { bs58 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import BN from 'bn.js';

import { ChangeLogEventV1 } from '../types';
import { accountCompressionEventBeet } from '../generated/types/AccountCompressionEvent';
import { ApplicationDataEvent, ChangeLogEventV1 as CLV1 } from '../generated';
import { PublicKey, TransactionResponse } from '@solana/web3.js';
import { SPL_NOOP_PROGRAM_ID } from '../constants';

/**
 * Helper method for indexing a {@link ConcurrentMerkleTree}
 * @param data
 * @returns
 */
export function deserializeChangeLogEventV1(data: Buffer): ChangeLogEventV1 {
  const event = accountCompressionEventBeet
    .toFixedFromData(data, 0)
    .read(data, 0);

  if (event.__kind == 'ChangeLog' && event.fields[0].__kind == 'V1') {
    const changeLogV1: CLV1 = event.fields[0].fields[0];
    return {
      treeId: changeLogV1.id,
      seq: new BN.BN(changeLogV1.seq),
      path: changeLogV1.path,
      index: changeLogV1.index,
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
export function deserializeApplicationDataEvent(
  data: Buffer
): ApplicationDataEvent {
  const event = accountCompressionEventBeet
    .toFixedFromData(data, 0)
    .read(data, 0);
  switch (event.__kind) {
    case 'ApplicationData': {
      return event.fields[0];
    }
    default:
      throw Error('Unable to decode buffer as ApplicationDataEvent');
  }
}

/**
 * Helper function to extract the ChangeLogEvent V1 events from a TransactionResponse
 * @param txResponse - TransactionResponse from the `@solana/web3.js`
 * @param programId - PublicKey of the program (aka `programId`) that utilized the leaf on the tree
 * @param noopProgramId - program id of the noop program used (default: `SPL_NOOP_PROGRAM_ID`)
 * @returns
 */ 
export function getChangeLogEventV1FromTransaction(
  txResponse: TransactionResponse,
  programId: PublicKey,
  noopProgramId: PublicKey = SPL_NOOP_PROGRAM_ID,
) : ChangeLogEventV1[]{
  // ensure a transaction response was provided
  if (!txResponse) throw Error("No txResponse provided");

  // find the correct index of the `programId` instruction
  const relevantIndex =
  txResponse.transaction.message.compiledInstructions.findIndex(
    (instruction) => {
      return (
        txResponse?.transaction.message.staticAccountKeys[
          instruction.programIdIndex
        ].toBase58() === programId.toBase58()
      );
    }
  );

  // locate the noop's inner instructions called via cpi from `programId`
  const relevantInnerIxs = txResponse!.meta?.innerInstructions?.[
    relevantIndex
  ].instructions.filter((instruction) => {
    return (
      txResponse?.transaction.message.staticAccountKeys[
        instruction.programIdIndex
      ].toBase58() === noopProgramId.toBase58()
    );
  });

  // when no valid noop instructions are found, throw an error
  if (!relevantInnerIxs || relevantInnerIxs.length == 0)
    throw Error('Unable to locate valid noop instructions');

  let changeLogEvents: ChangeLogEventV1[] = [];
  
  /**
   * note: the ChangeLogEvent V1 is expected to be at position `1`, 
   * and normally expect only 2 `relevantInnerIx`
   * so this sort method is more efficient for most uses cases
  */
  for (let i = relevantInnerIxs.length - 1; i > 0; i--) {
    try {
      changeLogEvents.push(deserializeChangeLogEventV1(
        Buffer.from(bs58.decode(relevantInnerIxs[i]?.data!))
      ))
    } catch (__) {
      // do nothing, invalid data is handled just after this for loop
    }
  }

  // when no changeLogEvents were found, throw an error
  if (changeLogEvents.length == 0)
    throw Error('Unable to locate any `ChangeLogEventV1` events');

  return changeLogEvents;
}