import * as borsh from 'borsh';
import { val } from '../../utils';
import { GumballMachineHeader, gumballMachineHeaderBeet } from '../src/generated/types/GumballMachineHeader';

/**
 * Manually create a model for GumballMachine accounts to deserialize manually
 */
export type OnChainGumballMachine = {
    header: GumballMachineHeader,
    configData: ConfigData
}

export const GUMBALL_MACHINE_HEADER_SIZE = 360;

/*type GumballMachineHeader = {
  urlBase: Buffer,              // [u8; 64]
  nameBase: Buffer,             // [u8; 32]
  symbol: Buffer,               // [u8; 8]
  sellerFeeBasisPoints: number, // u16
  isMutable: boolean,           // u8
  retainAuthority: boolean,     // u8
  _padding: Buffer,             // [u8; 4],
  price: BN,                    // u64
  goLiveDate: BN,               // i64
  mint: PublicKey,              
  botWallet: PublicKey,
  receiver: PublicKey,
  authority: PublicKey,
  collectionKey: PublicKey,
  creatorAddress: PublicKey,
  extensionLen: BN,             // usize
  maxMintSize: BN,              // u64
  remaining: BN,                // usize
  maxItems: BN,                 // u64
  totalItemsAdded: BN,          // usize
}*/

type ConfigData = {
  indexArray: Buffer,
  configLines: Buffer
}

// Deserialize on-chain gumball machine account to OnChainGumballMachine type
export function decodeGumballMachine(buffer: Buffer, accountSize: number): OnChainGumballMachine {
    let header: GumballMachineHeader;
    let postHeaderOffset: number;
    [header, postHeaderOffset] = gumballMachineHeaderBeet.deserialize(buffer);

    // Deserialize header
    // TODO(sorend): remove
    /*let header: GumballMachineHeader = {
      urlBase: Array.from(reader.readFixedArray(64)),
      nameBase: Array.from(reader.readFixedArray(32)),
      symbol: Array.from(reader.readFixedArray(8)),
      sellerFeeBasisPoints: reader.readU16(), 
      isMutable: reader.readU8(),
      retainAuthority: reader.readU8(),
      padding: Array.from(reader.readFixedArray(4)),
      price: reader.readU64(),
      goLiveDate: new BN(reader.readFixedArray(8), null, 'le'),
      mint: readPublicKey(reader),
      botWallet: readPublicKey(reader),
      receiver: readPublicKey(reader),
      authority: readPublicKey(reader),
      collectionKey: readPublicKey(reader),
      creatorAddress: readPublicKey(reader),
      extensionLen: new BN(reader.readFixedArray(8), null, 'le'),
      maxMintSize: reader.readU64(),
      remaining: new BN(reader.readFixedArray(8), null, 'le'),
      maxItems: reader.readU64(),
      totalItemsAdded: new BN(reader.readFixedArray(8), null, 'le'),
    };*/

    // Deserailize indices and config section
    let reader = new borsh.BinaryReader(Buffer.from(buffer));
    // TODO(sorend): nicer would be nice but the Buffer API is not the best
    reader.readFixedArray(postHeaderOffset);
    let numIndexArrayBytes = 4 * val(header.maxItems).toNumber();
    let numConfigBytes = val(header.extensionLen).toNumber() * val(header.maxItems).toNumber();
    let configData: ConfigData = {
      indexArray: Buffer.from(reader.readFixedArray(numIndexArrayBytes)),
      configLines: Buffer.from(reader.readFixedArray(numConfigBytes)),
    }

    if (accountSize != reader.offset) {
        throw new Error("Reader processed different number of bytes than account size")
    }
    return {
        header,
        configData
    }
}