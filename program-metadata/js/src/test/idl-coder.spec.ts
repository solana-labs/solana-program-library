import { Keypair, PublicKey } from "@solana/web3.js";
import { IdlCoder } from "../idl/idl-coder";
import { updateMetadataEntryIx } from "../program/instruction";
import { expect } from "chai";

require("dotenv").config();

const idl = require("./idl.json");

function getRandomPublicKey(): PublicKey {
  const acct = Keypair.generate();
  return acct.publicKey;
}

describe("IDL Coder", async () => {
  const programId = getRandomPublicKey();
  const classKey = getRandomPublicKey();
  const nameKey = getRandomPublicKey();
  const targetProgramKey = getRandomPublicKey();
  const targetProgramDataKey = getRandomPublicKey();
  const targetProgramAuthorityKey = getRandomPublicKey();
  const nameServiceKey = getRandomPublicKey();

  it("should decode a Borsh instruction", async () => {
    const value = "test value";
    const coder = new IdlCoder(idl);

    const transactionInstruction = updateMetadataEntryIx(
      programId,
      classKey,
      nameKey,
      targetProgramKey,
      targetProgramDataKey,
      targetProgramAuthorityKey,
      nameServiceKey,
      value
    );

    const decoded = coder.decodeInstruction(transactionInstruction);
    expect(decoded.programId).to.equal(programId);
    expect(decoded.formattedName).to.equal("Update Metadata Entry");
    expect(decoded.accounts.length).to.equal(6);
    expect(decoded.args.length).to.equal(1);
    expect(
      "formattedName" in decoded.accounts[2] &&
        decoded.accounts[2].formattedName
    ).to.equal("Target Program");
    expect("value" in decoded.args[0] && decoded.args[0].value).to.equal(value);
    expect("value" in decoded.args[0] && decoded.args[0].type).to.equal(
      "string"
    );
  });
});
