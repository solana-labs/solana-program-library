"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const web3_js_1 = require("@solana/web3.js");
async function test() {
    const privKey = web3_js_1.Keypair.fromSecretKey(new Uint8Array([100, 250, 205, 42, 30, 129, 211, 123, 14, 140, 159, 196, 253, 68, 243, 110, 49, 127, 142, 210, 125, 128, 217, 186, 240, 221, 175, 187, 58, 239, 107, 64, 108, 45, 162, 64, 173, 213, 98, 216, 201, 181, 107, 38, 101, 69, 101, 19, 139, 83, 224, 219, 89, 70, 49, 45, 175, 121, 69, 112, 28, 67, 251, 152]));
    const connection = new web3_js_1.Connection('http://localhost:8899');
    const targetProgramId = new web3_js_1.PublicKey('CCgAr385ZqyH5CWHTnekPRFKgCpS9P2SJG2Qcg6YVBiv');
    // const ix = await createMetadataEntry(
    //   connection,
    //   targetProgramId,
    //   privKey.publicKey,
    //   privKey.publicKey,
    //   'foo',
    //   'test'
    // );
    // const tx = new Transaction();
    // const signers: Keypair[] = [];
    // signers.push(privKey);
    // tx.add(ix);
    // try {
    //   let res = await connection.sendTransaction(tx, signers, {
    //     preflightCommitment: 'single'
    //   });
    //   console.log(res);
    // } catch (error) {
    //   console.log(error);
    // }
    // const ix = await updateMetadataEntry(
    //   connection,
    //   targetProgramId,
    //   privKey.publicKey,
    //   'foo',
    //   'test2'
    // );
    const ix = await deleteMetadataEntry(connection, targetProgramId, privKey.publicKey, privKey.publicKey, 'foo');
    const tx = new web3_js_1.Transaction();
    const signers = [];
    signers.push(privKey);
    tx.add(ix);
    try {
        let res = await connection.sendTransaction(tx, signers, {
            preflightCommitment: 'single'
        });
        console.log(res);
    }
    catch (error) {
        console.log(error);
    }
}
test();
//# sourceMappingURL=test.js.map