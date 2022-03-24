import * as anchor from "@project-serum/anchor";
import { Program, ProgramError } from "@project-serum/anchor";
import { RecordAnchor } from "../target/types/record_anchor";
import * as assert from 'assert';

describe("record-anchor", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.Provider.local();
  anchor.setProvider(provider);

  const program = anchor.workspace.RecordAnchor as Program<RecordAnchor>;
  
  const initializeStorageAccount = async (recordAccount, data) => {
    const tx = await program.rpc.initialize({
      accounts: {
        recordAccount: recordAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId
      },
      signers: [recordAccount]
    });

    let offset = new anchor.BN(0);
    const tx1 = await program.rpc.write(offset, Buffer.from(data),{
      accounts: {
        recordAccount: recordAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId
      }
    });
  }

  it("Initialize Success ", async () => {
    const recordAccount = anchor.web3.Keypair.generate();
    let bytes = Uint8Array.from([0xf0, 0x9f, 0x90, 0x86, 0x86, 0x86, 0x86, 0x86]);
    await initializeStorageAccount(recordAccount, bytes);
  });

  it("Initialize Twice Failed ", async () => {
    const recordAccount = anchor.web3.Keypair.generate();
    let bytes = Uint8Array.from([0xf0, 0x9f, 0x90, 0x86, 0x86, 0x86, 0x86, 0x86]);
    await initializeStorageAccount(recordAccount, bytes);

    assert.rejects(async () => {
      await program.rpc.initialize({
        accounts: {
          recordAccount: recordAccount.publicKey,
          authority: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId
        },
        signers: [recordAccount]
      });
    }, new Error("Account already initialized!"));
  });

  it("Write Success ", async () => {
    const recordAccount = anchor.web3.Keypair.generate();
    let bytes = Uint8Array.from([0xf0, 0x9f, 0x90, 0x86, 0x86, 0x86, 0x86, 0x86]);
    await initializeStorageAccount(recordAccount, bytes);

    let offset = new anchor.BN(0);

    let newData = Uint8Array.from([0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0])
    
    const tx = await program.rpc.write(offset, Buffer.from(newData), {
      accounts: {
        recordAccount: recordAccount.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId
      }
    })

    const accountData = await program.account.recordData.fetch(recordAccount.publicKey) as any;
    assert.equal(Array.from(newData).toString(), accountData.data.toString());
  });

  it("Write Failed Wrong Authority! ", async () => {
    const recordAccount = anchor.web3.Keypair.generate();
    let bytes = Uint8Array.from([0xf0, 0x9f, 0x90, 0x86, 0x86, 0x86, 0x86, 0x86]);
    await initializeStorageAccount(recordAccount, bytes);

    let offset = new anchor.BN(0);

    let newData = Uint8Array.from([0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0])
    const wrongAuthority = anchor.web3.Keypair.generate();

    assert.rejects(async () => {
      await program.rpc.write(offset, Buffer.from(newData), {
        accounts: {
          recordAccount: recordAccount.publicKey,
          authority: wrongAuthority.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId
        }
      });
    }, new Error("Incorrect Authority"));
  });

  it("Set Authority Success ", async () => {
    const recordAccount = anchor.web3.Keypair.generate();
    let bytes = Uint8Array.from([0xf0, 0x9f, 0x90, 0x86, 0x86, 0x86, 0x86, 0x86]);
    await initializeStorageAccount(recordAccount, bytes);

    const newAuthority = anchor.web3.Keypair.generate();

    const tx = await program.rpc.setAuthority({
      accounts: {
        recordAccount: recordAccount.publicKey,
        authority: provider.wallet.publicKey,
        newAuthority: newAuthority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId
      }
    })

    const accountData = await program.account.recordData.fetch(recordAccount.publicKey) as any;
    assert.equal(accountData.authority.toString(), newAuthority.publicKey.toString())
  });

  it("Close Account Success ", async () => {
    const recordAccount = anchor.web3.Keypair.generate();
    let bytes = Uint8Array.from([0xf0, 0x9f, 0x90, 0x86, 0x86, 0x86, 0x86, 0x86]);
    await initializeStorageAccount(recordAccount, bytes);

    const reciever = anchor.web3.Keypair.generate();
    const tx = await program.rpc.closeAccount({
      accounts: {
        recordAccount: recordAccount.publicKey,
        authority: provider.wallet.publicKey,
        reciever: reciever.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId
      }
    })

    assert.ok(true);
  });

  it("Close Account Fail Wrong Authority ", async () => {
    const recordAccount = anchor.web3.Keypair.generate();
    let bytes = Uint8Array.from([0xf0, 0x9f, 0x90, 0x86, 0x86, 0x86, 0x86, 0x86]);
    await initializeStorageAccount(recordAccount, bytes);

    const reciever = anchor.web3.Keypair.generate();
    const wrongAuthority = anchor.web3.Keypair.generate();

    assert.rejects(async () => {
      await program.rpc.closeAccount({
        accounts: {
          recordAccount: recordAccount.publicKey,
          authority: wrongAuthority,
          reciever: reciever.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId
        }
      })
    }, new Error("Incorrect Authority"));
  });
});
