import { BN } from "@project-serum/anchor";
import {
  PublicKey,
} from "@solana/web3.js";

export { GumballMachine } from "../../../target/types/gumball_machine";

// @notice: This type is only used to facilitate creating the initialize_gumball_machine instruction on the client side,
//          it is not related to deserailizing onchain GumballMachine accounts. See ../instructions/index.ts for usage
export type InitGumballMachineProps = {
    urlBase: Buffer,
    nameBase: Buffer,
    symbol: Buffer,
    sellerFeeBasisPoints: number,
    isMutable: boolean,
    retainAuthority: boolean,
    price: BN,
    goLiveDate: BN,
    mint: PublicKey,
    botWallet: PublicKey,
    receiver: PublicKey,
    authority: PublicKey,
    collectionKey: PublicKey,
    creatorAddress: PublicKey,
    extensionLen: BN,
    maxMintSize: BN, 
    maxItems: BN
}