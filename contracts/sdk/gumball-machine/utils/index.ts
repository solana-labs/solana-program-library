import {
  PublicKey
} from "@solana/web3.js";

export async function getBubblegumAuthorityPDAKey(merkleRollPubKey: PublicKey, bubblegumProgramId: PublicKey) {
    const [bubblegumAuthorityPDAKey] = await PublicKey.findProgramAddress(
      [merkleRollPubKey.toBuffer()],
      bubblegumProgramId
    );
    return bubblegumAuthorityPDAKey;
  }

export async function getWillyWonkaPDAKey(gumballMachinePubkey: PublicKey, gumballMachineProgramId: PublicKey) {
    const [willyWonkaPDAKey] = await PublicKey.findProgramAddress(
        [gumballMachinePubkey.toBuffer()],
        gumballMachineProgramId
    );
    return willyWonkaPDAKey;
}