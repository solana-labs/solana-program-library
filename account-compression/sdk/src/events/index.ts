import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import BN from "bn.js";

import { ChangeLogEventV1 } from "../types";
import { accountCompressionEventBeet } from "../generated/types/AccountCompressionEvent";
import { ApplicationDataEvent, ChangeLogEventV1 as CLV1 } from "../generated";
import {
  PublicKey,
  TransactionResponse,
  VersionedTransactionResponse,
} from "@solana/web3.js";
import { SPL_NOOP_PROGRAM_ID } from "../constants";

/**
 * Helper method for indexing a {@link ConcurrentMerkleTree}
 * @param data
 * @returns
 */
export function deserializeChangeLogEventV1(data: Buffer): ChangeLogEventV1 {
  const event = accountCompressionEventBeet
    .toFixedFromData(data, 0)
    .read(data, 0);

  if (event.__kind == "ChangeLog" && event.fields[0].__kind == "V1") {
    const changeLogV1: CLV1 = event.fields[0].fields[0];
    return {
      treeId: changeLogV1.id,
      seq: new BN.BN(changeLogV1.seq),
      path: changeLogV1.path,
      index: changeLogV1.index,
    };
  } else {
    throw Error("Unable to decode buffer as ChangeLogEvent V1");
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
    case "ApplicationData": {
      return event.fields[0];
    }
    default:
      throw Error("Unable to decode buffer as ApplicationDataEvent");
  }
}

/**
 * Helper function to extract the all ChangeLogEventV1 emitted in a transaction
 * @param txResponse - Transaction response from `@solana/web3.js`
 * @param noopProgramId - program id of the noop program used (default: `SPL_NOOP_PROGRAM_ID`)
 * @returns
 */
export function getAllChangeLogEventV1FromTransaction(
  txResponse: TransactionResponse | VersionedTransactionResponse,
  noopProgramId: PublicKey = SPL_NOOP_PROGRAM_ID
): ChangeLogEventV1[] {
  // ensure a transaction response was provided
  if (!txResponse) throw Error("No txResponse provided");

  // flatten the array of all account keys (e.g. static, readonly, writable)
  const accountKeys = txResponse.transaction.message
    .getAccountKeys()
    .keySegments()
    .flat();

  let changeLogEvents: ChangeLogEventV1[] = [];

  // locate and parse noop instruction calls via cpi (aka inner instructions)
  txResponse!.meta?.innerInstructions?.forEach((compiledIx) => {
    compiledIx.instructions.forEach((innerIx) => {
      // only attempt to parse noop instructions
      if (
        noopProgramId.toBase58() !==
        accountKeys[innerIx.programIdIndex].toBase58()
      )
        return;

      try {
        // try to deserialize the cpi data as a changelog event
        changeLogEvents.push(
          deserializeChangeLogEventV1(Buffer.from(bs58.decode(innerIx.data)))
        );
      } catch (__) {
        // this noop cpi is not a changelog event. do nothing with it.
      }
    });
  });

  return changeLogEvents;
}
