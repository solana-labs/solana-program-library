const anchor = require("@project-serum/anchor");
const assert = require("assert");

describe("memo", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const program = anchor.workspace.Memo;

  it("Test 1 => Sending Buffers", async () => {
    const string = "letters";
    var codedString = Buffer.from(string);

    const tx = await program.rpc.buildMemo(codedString);
    console.log("Transaction Signature:", tx);
  });

  it("Test 2 => Asserting Emojis and Bytes", async () => {
    let emoji = Uint8Array.from(Buffer.from("🐆"));
    let bytes = Uint8Array.from([0xf0, 0x9f, 0x90, 0x86]);
    assert.equal(emoji.toString(), bytes.toString());

    const tx1 = await program.rpc.buildMemo(Buffer.from("🐆"));

    const tx2 = await program.rpc.buildMemo(Buffer.from(bytes));
    console.log("Transaction Signature One:", tx1);
    console.log("Transaction Signature Two:", tx2);
  });

  it("Test 3 => Sending Signed Transaction", async () => {
    const pubkey1 = anchor.web3.Keypair.generate();
    const pubkey2 = anchor.web3.Keypair.generate();
    const pubkey3 = anchor.web3.Keypair.generate();

    const tx = await program.rpc.buildMemo(Buffer.from("🐆"), {
      accounts: [],
      remainingAccounts: [
        { pubkey: pubkey1.publicKey, isWritable: false, isSigner: true },
        { pubkey: pubkey2.publicKey, isWritable: false, isSigner: true },
        { pubkey: pubkey3.publicKey, isWritable: false, isSigner: true },
      ],
      signers: [pubkey1, pubkey2, pubkey3],
    });
    console.log("Transaction Signature:", tx);
  });

  it("Test 4 => Sending Unsigned Transaction with a Memo", async () => {
    // This test should fail because the transaction is not signed.
    const pubkey1 = anchor.web3.Keypair.generate();
    const pubkey2 = anchor.web3.Keypair.generate();
    const pubkey3 = anchor.web3.Keypair.generate();

    assert.rejects(() => {
      program.rpc.buildMemo(Buffer.from("🐆"), {
        accounts: [],
        remainingAccounts: [
          { pubkey: pubkey1.publicKey, isWritable: false, isSigner: false },
          { pubkey: pubkey2.publicKey, isWritable: false, isSigner: false },
          { pubkey: pubkey3.publicKey, isWritable: false, isSigner: false },
        ],
        signers: [pubkey1, pubkey2, pubkey3],
      });
    }, new Error("unknown signer"));

    console.log("Test failed successfully :)");
  });

  it("Test 5 => Sending a transaction with missing signers", async () => {
    // This test should fail because the transaction is not signed completely.
    const pubkey1 = anchor.web3.Keypair.generate();
    const pubkey2 = anchor.web3.Keypair.generate();
    const pubkey3 = anchor.web3.Keypair.generate();

    assert.rejects(async () => {
      await program.rpc.buildMemo(Buffer.from("🐆"), {
        accounts: [],
        remainingAccounts: [
          { pubkey: pubkey1.publicKey, isWritable: false, isSigner: true },
          { pubkey: pubkey2.publicKey, isWritable: false, isSigner: false },
          { pubkey: pubkey3.publicKey, isWritable: false, isSigner: true },
        ],
        signers: [pubkey1, pubkey2, pubkey3],
      });
    }, new Error("unknown signer"));
    console.log("Test failed successfully :)");
  });

  it("Test 6 => Testing invalid input", async () => {
    // This test should fail because the input is invalid.
    let invalid_utf8 = Uint8Array.from([
      0xf0, 0x9f, 0x90, 0x86, 0xf0, 0x9f, 0xff, 0x86,
    ]);

    assert.rejects(() => {
      program.rpc.buildMemo(Buffer.from(invalid_utf8));
    }, new Error());
    console.log("Test failed successfully :)");
  });
});
