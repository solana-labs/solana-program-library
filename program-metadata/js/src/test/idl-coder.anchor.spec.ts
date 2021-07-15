import { Keypair, PublicKey, TransactionInstruction } from "@solana/web3.js";
import { IdlCoder } from "../idl/idl-coder";
import { generateAnchorInstruction } from "./anchor/generate-anchor-instruction";
import { Idl } from "../idl/idl";
import { expect } from "chai";

function getRandomPublicKey(): PublicKey {
  return Keypair.generate().publicKey;
}

describe("IDL Coder - Anchor", async () => {
  const idl: Idl = require("./test-idl-anchor.json");
  const programId = getRandomPublicKey();
  const counterKey = getRandomPublicKey();
  const rentKey = getRandomPublicKey();
  const authorityKey = getRandomPublicKey();

  it("should decode an Anchor instruction", async () => {
    const coder = new IdlCoder(idl);

    const ixData = generateAnchorInstruction("create", {
      authority: authorityKey,
    });

    const ix = new TransactionInstruction({
      programId: programId,
      keys: [
        { pubkey: counterKey, isSigner: false, isWritable: true },
        { pubkey: rentKey, isSigner: false, isWritable: false },
      ],
      data: ixData,
    });

    const decoded = coder.decodeInstruction(ix);
    expect(decoded.programId).to.equal(programId);
    expect(decoded.formattedName).to.equal("Create");
    expect(decoded.accounts.length).to.equal(2);
    expect(decoded.args.length).to.equal(1);
    expect(
      "formattedName" in decoded.accounts[1] &&
        decoded.accounts[1].formattedName
    ).to.equal("Rent");
    expect(
      "value" in decoded.args[0] && decoded.args[0].value.toBase58()
    ).to.equal(authorityKey.toBase58());
  });
});
