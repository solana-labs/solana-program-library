import {
    PublicKey,
} from "@solana/web3.js";
import { BN } from "@project-serum/anchor";

export async function getListingPDAKeyForPrice(price: BN, SugarShackProgramid: PublicKey): Promise<PublicKey> {
    const [key] = await PublicKey.findProgramAddress(
      [price.toArrayLike(Buffer,"le",8)],
      SugarShackProgramid
    );
    return key;
}