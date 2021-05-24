import {
  Connection,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from '@solana/web3.js';

import { deleteNameRegistry, NAME_PROGRAM_ID } from './bindings';
import {
  createInstruction,
  deleteInstruction,
  transferInstruction,
  updateInstruction,
} from './instructions';
import { NameRegistryState } from './state';
import {
  getFilteredProgramAccounts,
  getHashedName,
  getNameAccountKey,
  Numberu32,
  Numberu64,
} from './utils';
import { deserialize, deserializeUnchecked, Schema, serialize } from 'borsh';

////////////////////////////////////////////////////
// Global Variables

export const TWITTER_VERIFICATION_AUTHORITY = new PublicKey(
  'FvPH7PrVrLGKPfqaf3xJodFTjZriqrAXXLTVWEorTFBi'
);
// The address of the name registry that will be a parent to all twitter handle registries,
// it should be owned by the TWITTER_VERIFICATION_AUTHORITY and it's name is irrelevant
export const TWITTER_ROOT_PARENT_REGISTRY_KEY = new PublicKey(
  '4YcexoW3r78zz16J2aqmukBLRwGq6rAvWzJpkYAXqebv'
);

////////////////////////////////////////////////////
// Bindings

// Signed by the authority, the payer and the verified pubkey
export async function createVerifiedTwitterRegistry(
  connection: Connection,
  twitterHandle: string,
  verifiedPubkey: PublicKey,
  space: number, // The space that the user will have to write data into the verified registry
  payerKey: PublicKey
): Promise<TransactionInstruction[]> {
  // Create user facing registry
  const hashedTwitterHandle = await getHashedName(twitterHandle);
  const twitterHandleRegistryKey = await getNameAccountKey(
    hashedTwitterHandle,
    undefined,
    TWITTER_ROOT_PARENT_REGISTRY_KEY
  );

  let instructions = [
    createInstruction(
      NAME_PROGRAM_ID,
      SystemProgram.programId,
      twitterHandleRegistryKey,
      verifiedPubkey,
      payerKey,
      hashedTwitterHandle,
      new Numberu64(await connection.getMinimumBalanceForRentExemption(space)),
      new Numberu32(space),
      undefined,
      TWITTER_ROOT_PARENT_REGISTRY_KEY,
      TWITTER_VERIFICATION_AUTHORITY // Twitter authority acts as owner of the parent for all user-facing registries
    ),
  ];

  instructions = instructions.concat(
    await createReverseTwitterRegistry(
      connection,
      twitterHandle,
      twitterHandleRegistryKey,
      verifiedPubkey,
      payerKey
    )
  );

  return instructions;
}

// Overwrite the data that is written in the user facing registry
// Signed by the verified pubkey
export async function changeTwitterRegistryData(
  twitterHandle: string,
  verifiedPubkey: PublicKey,
  offset: number, // The offset at which to write the input data into the NameRegistryData
  input_data: Buffer
): Promise<TransactionInstruction[]> {
  const hashedTwitterHandle = await getHashedName(twitterHandle);
  const twitterHandleRegistryKey = await getNameAccountKey(
    hashedTwitterHandle,
    undefined,
    TWITTER_ROOT_PARENT_REGISTRY_KEY
  );

  const instructions = [
    updateInstruction(
      NAME_PROGRAM_ID,
      twitterHandleRegistryKey,
      new Numberu32(offset),
      input_data,
      verifiedPubkey
    ),
  ];

  return instructions;
}

// Change the verified pubkey for a given twitter handle
// Signed by the Authority, the verified pubkey and the payer
export async function changeVerifiedPubkey(
  connection: Connection,
  twitterHandle: string,
  currentVerifiedPubkey: PublicKey,
  newVerifiedPubkey: PublicKey,
  payerKey: PublicKey
): Promise<TransactionInstruction[]> {
  const hashedTwitterHandle = await getHashedName(twitterHandle);
  const twitterHandleRegistryKey = await getNameAccountKey(
    hashedTwitterHandle,
    undefined,
    TWITTER_ROOT_PARENT_REGISTRY_KEY
  );

  // Transfer the user-facing registry ownership
  let instructions = [
    transferInstruction(
      NAME_PROGRAM_ID,
      twitterHandleRegistryKey,
      newVerifiedPubkey,
      currentVerifiedPubkey,
      undefined
    ),
  ];

  // Delete the current reverse registry
  const currentHashedVerifiedPubkey = await getHashedName(
    currentVerifiedPubkey.toString()
  );
  const currentReverseRegistryKey = await getNameAccountKey(
    currentHashedVerifiedPubkey,
    TWITTER_VERIFICATION_AUTHORITY,
    undefined
  );
  instructions.push(
    await deleteNameRegistry(
      connection,
      currentVerifiedPubkey.toString(),
      payerKey,
      TWITTER_VERIFICATION_AUTHORITY,
      TWITTER_ROOT_PARENT_REGISTRY_KEY
    )
  );

  // Create the new reverse registry
  instructions = instructions.concat(
    await createReverseTwitterRegistry(
      connection,
      twitterHandle,
      twitterHandleRegistryKey,
      newVerifiedPubkey,
      payerKey
    )
  );

  return instructions;
}

// Delete the verified registry for a given twitter handle
// Signed by the verified pubkey
export async function deleteTwitterRegistry(
  twitterHandle: string,
  verifiedPubkey: PublicKey
): Promise<TransactionInstruction[]> {
  const hashedTwitterHandle = await getHashedName(twitterHandle);
  const twitterHandleRegistryKey = await getNameAccountKey(
    hashedTwitterHandle,
    undefined,
    TWITTER_ROOT_PARENT_REGISTRY_KEY
  );

  const hashedVerifiedPubkey = await getHashedName(verifiedPubkey.toString());
  const reverseRegistryKey = await getNameAccountKey(
    hashedVerifiedPubkey,
    TWITTER_VERIFICATION_AUTHORITY,
    TWITTER_ROOT_PARENT_REGISTRY_KEY
  );

  const instructions = [
    // Delete the user facing registry
    deleteInstruction(
      NAME_PROGRAM_ID,
      twitterHandleRegistryKey,
      verifiedPubkey,
      verifiedPubkey
    ),
    // Delete the reverse registry
    deleteInstruction(
      NAME_PROGRAM_ID,
      reverseRegistryKey,
      verifiedPubkey,
      verifiedPubkey
    ),
  ];

  return instructions;
}

//////////////////////////////////////////
// Getter Functions

// Returns the key of the user-facing registry
export async function getTwitterRegistryKey(
  twitter_handle: string
): Promise<PublicKey> {
  const hashedTwitterHandle = await getHashedName(twitter_handle);
  return await getNameAccountKey(
    hashedTwitterHandle,
    undefined,
    TWITTER_ROOT_PARENT_REGISTRY_KEY
  );
}

export async function getTwitterRegistry(
  connection: Connection,
  twitter_handle: string
): Promise<NameRegistryState> {
  const hashedTwitterHandle = await getHashedName(twitter_handle);
  const twitterHandleRegistryKey = await getNameAccountKey(
    hashedTwitterHandle,
    undefined,
    TWITTER_ROOT_PARENT_REGISTRY_KEY
  );
  const registry = NameRegistryState.retrieve(
    connection,
    twitterHandleRegistryKey
  );
  return registry;
}

export async function getHandleAndRegistryKey(
  connection: Connection,
  verifiedPubkey: PublicKey
): Promise<[string, PublicKey]> {
  const hashedVerifiedPubkey = await getHashedName(verifiedPubkey.toString());
  const reverseRegistryKey = await getNameAccountKey(
    hashedVerifiedPubkey,
    TWITTER_VERIFICATION_AUTHORITY,
    TWITTER_ROOT_PARENT_REGISTRY_KEY
  );

  let reverseRegistryState = await ReverseTwitterRegistryState.retrieve(
    connection,
    reverseRegistryKey
  );
  return [
    reverseRegistryState.twitterHandle,
    new PublicKey(reverseRegistryState.twitterRegistryKey),
  ];
}

// Uses the RPC node filtering feature, execution speed may vary
export async function getTwitterHandleandRegistryKeyViaFilters(
  connection: Connection,
  verifiedPubkey: PublicKey
): Promise<[string, PublicKey]> {
  const filters = [
    {
      memcmp: {
        offset: 0,
        bytes: TWITTER_ROOT_PARENT_REGISTRY_KEY.toBase58(),
      },
    },
    {
      memcmp: {
        offset: 32,
        bytes: verifiedPubkey.toBase58(),
      },
    },
    {
      memcmp: {
        offset: 64,
        bytes: TWITTER_VERIFICATION_AUTHORITY.toBase58(),
      },
    },
  ];

  const filteredAccounts = await getFilteredProgramAccounts(
    connection,
    NAME_PROGRAM_ID,
    filters
  );

  for (const f of filteredAccounts) {
    if (f.accountInfo.data.length > NameRegistryState.HEADER_LEN + 32) {
      let data = f.accountInfo.data.slice(NameRegistryState.HEADER_LEN);
      let state: ReverseTwitterRegistryState = deserialize(
        ReverseTwitterRegistryState.schema,
        ReverseTwitterRegistryState,
        data
      );
      return [state.twitterHandle, new PublicKey(state.twitterRegistryKey)];
    }
  }
  throw new Error('Registry not found.');
}

// Uses the RPC node filtering feature, execution speed may vary
// Does not give you the handle, but is an alternative to getHandlesAndKeysFromVerifiedPubkey + getTwitterRegistry to get the data
export async function getTwitterRegistryData(
  connection: Connection,
  verifiedPubkey: PublicKey
): Promise<Buffer> {
  const filters = [
    {
      memcmp: {
        offset: 0,
        bytes: TWITTER_ROOT_PARENT_REGISTRY_KEY.toBytes(),
      },
    },
    {
      memcmp: {
        offset: 32,
        bytes: verifiedPubkey.toBytes(),
      },
    },
    {
      memcmp: {
        offset: 64,
        bytes: new PublicKey(Buffer.alloc(32, 0)).toBase58(),
      },
    },
  ];

  const filteredAccounts = await getFilteredProgramAccounts(
    connection,
    NAME_PROGRAM_ID,
    filters
  );

  if (filteredAccounts.length > 1) {
    throw new Error('Found more than one registry.');
  }

  return filteredAccounts[0].accountInfo.data.slice(
    NameRegistryState.HEADER_LEN
  );
}

//////////////////////////////////////////////
// Utils

export class ReverseTwitterRegistryState {
  twitterRegistryKey: Uint8Array;
  twitterHandle: string;

  static schema: Schema = new Map([
    [
      ReverseTwitterRegistryState,
      {
        kind: 'struct',
        fields: [
          ['twitterRegistryKey', [32]],
          ['twitterHandle', 'string'],
        ],
      },
    ],
  ]);
  constructor(obj: { twitterRegistryKey: Uint8Array; twitterHandle: string }) {
    this.twitterRegistryKey = obj.twitterRegistryKey;
    this.twitterHandle = obj.twitterHandle;
  }

  public static async retrieve(
    connection: Connection,
    reverseTwitterAccountKey: PublicKey
  ): Promise<ReverseTwitterRegistryState> {
    let reverseTwitterAccount = await connection.getAccountInfo(
      reverseTwitterAccountKey,
      'processed'
    );
    if (!reverseTwitterAccount) {
      throw new Error('Invalid reverse Twitter account provided');
    }

    let res: ReverseTwitterRegistryState = deserializeUnchecked(
      this.schema,
      ReverseTwitterRegistryState,
      reverseTwitterAccount.data.slice(NameRegistryState.HEADER_LEN)
    );

    return res;
  }
}

export async function createReverseTwitterRegistry(
  connection: Connection,
  twitterHandle: string,
  twitterRegistryKey: PublicKey,
  verifiedPubkey: PublicKey,
  payerKey: PublicKey
): Promise<TransactionInstruction[]> {
  // Create the reverse lookup registry
  const hashedVerifiedPubkey = await getHashedName(verifiedPubkey.toString());
  const reverseRegistryKey = await getNameAccountKey(
    hashedVerifiedPubkey,
    TWITTER_VERIFICATION_AUTHORITY,
    TWITTER_ROOT_PARENT_REGISTRY_KEY
  );
  let reverseTwitterRegistryStateBuff = serialize(
    ReverseTwitterRegistryState.schema,
    new ReverseTwitterRegistryState({
      twitterRegistryKey: twitterRegistryKey.toBytes(),
      twitterHandle,
    })
  );
  return [
    createInstruction(
      NAME_PROGRAM_ID,
      SystemProgram.programId,
      reverseRegistryKey,
      verifiedPubkey,
      payerKey,
      hashedVerifiedPubkey,
      new Numberu64(
        await connection.getMinimumBalanceForRentExemption(
          reverseTwitterRegistryStateBuff.length
        )
      ),
      new Numberu32(reverseTwitterRegistryStateBuff.length),
      TWITTER_VERIFICATION_AUTHORITY, // Twitter authority acts as class for all reverse-lookup registries
      TWITTER_ROOT_PARENT_REGISTRY_KEY, // Reverse registries are also children of the root
      TWITTER_VERIFICATION_AUTHORITY
    ),
    updateInstruction(
      NAME_PROGRAM_ID,
      reverseRegistryKey,
      new Numberu32(0),
      Buffer.from(reverseTwitterRegistryStateBuff),
      TWITTER_VERIFICATION_AUTHORITY
    ),
  ];
}
