import { serialize } from "@bonfida/borsh-js";
import { Connection, Account, PublicKey, AccountInfo } from "@solana/web3.js";
import { transferNameOwnership, updateNameRegistryData, createNameRegistry, deleteNameRegistry } from "./bindings";
import { readFile } from "fs/promises";
import { Numberu64, signAndSendTransactionInstructions } from "./utils";
import { sign } from "tweetnacl";
import { getHashedName, getNameAccountKey, Numberu32 } from ".";
import { NameRegistryState } from "./state";

const ENDPOINT = 'https://devnet.solana.com/';
// const ENDPOINT = 'https://solana-api.projectserum.com/';

export async function test() {
  let connection = new Connection(ENDPOINT);
  let secretKey = JSON.parse(
    (await readFile('/home/lcchy-work/.config/solana/id_devnet.json')).toString()
  );
  let adminAccount = new Account(secretKey);

  let root_name = ".sol";

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

  let hashed_root_name = await getHashedName(root_name);
  let nameAccountKey = await getNameAccountKey(hashed_root_name);
  console.log(await NameRegistryState.retrieve(connection, nameAccountKey));
}

test();
