import { readFile } from 'fs/promises';

import { AccountInfo, Connection, Keypair, PublicKey } from '@solana/web3.js';
import { serialize } from 'borsh';
import { sign } from 'tweetnacl';

import {
  createNameRegistry,
  deleteNameRegistry,
  transferNameOwnership,
  updateNameRegistryData,
} from './bindings';
import { NameRegistryState } from './state';
import {
  getHashedName,
  getNameAccountKey,
  Numberu32,
  Numberu64,
  signAndSendTransactionInstructions,
} from './utils';

const ENDPOINT = 'https://devnet.solana.com/';
// const ENDPOINT = 'https://solana-api.projectserum.com/';

export async function test() {
  const connection = new Connection(ENDPOINT);
  // let secretKey = JSON.parse(
  //   (await readFile('/home/lcchy-work/.config/solana/id_devnet.json')).toString()
  // );
  // let adminAccount = new Keypair(secretKey);

  const root_name = '.sol';

  // let create_instruction = await createNameRegistry(
  //   connection,
  //   root_name,
  //   1000,
  //   adminAccount.publicKey,
  //   adminAccount.publicKey,
  // );

  // console.log(
  //   await signAndSendTransactionInstructions(
  //     connection,
  //     [adminAccount],
  //     adminAccount,
  //     [create_instruction]
  //   )
  // );

  // let input_data = Buffer.from("Du");
  // let updateInstruction = await updateNameRegistryData(
  //   connection,
  //   root_name,
  //   0,
  //   input_data,
  // );

  // console.log(
  //   await signAndSendTransactionInstructions(
  //     connection,
  //     [adminAccount],
  //     adminAccount,
  //     [updateInstruction]
  //   )
  // );

  // let transferInstruction = await transferNameOwnership(
  //   connection,
  //   root_name,
  //   adminAccount.publicKey,
  //   adminAccount.publicKey,
  // );

  // console.log(
  //   await signAndSendTransactionInstructions(
  //     connection,
  //     [adminAccount],
  //     adminAccount,
  //     [transferInstruction]
  //   )
  // );

  // let deleteInstruction = await deleteNameRegistry(
  //   connection,
  //   root_name,
  //   adminAccount.publicKey
  // );

  // console.log(
  //   await signAndSendTransactionInstructions(
  //     connection,
  //     [adminAccount],
  //     adminAccount,
  //     [deleteInstruction]
  //   )
  // );

  const hashed_root_name = await getHashedName(root_name);
  const nameAccountKey = await getNameAccountKey(hashed_root_name);
  console.log(await NameRegistryState.retrieve(connection, nameAccountKey));
}

test();
