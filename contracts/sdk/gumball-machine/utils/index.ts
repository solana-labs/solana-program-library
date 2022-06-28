import {
  PublicKey
} from "@solana/web3.js";

export async function getWillyWonkaPDAKey(gumballMachinePubkey: PublicKey, gumballMachineProgramId: PublicKey) {
    const [willyWonkaPDAKey] = await PublicKey.findProgramAddress(
        [gumballMachinePubkey.toBuffer()],
        gumballMachineProgramId
    );
    return willyWonkaPDAKey;
}