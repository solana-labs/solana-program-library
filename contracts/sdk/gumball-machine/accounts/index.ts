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

type ConfigData = {
  indexArray: Buffer,
  configLines: Buffer
}

// Deserialize on-chain gumball machine account to OnChainGumballMachine type
export function decodeGumballMachine(buffer: Buffer, accountSize: number): OnChainGumballMachine {
    let header: GumballMachineHeader;
    let postHeaderOffset: number;
    [header, postHeaderOffset] = gumballMachineHeaderBeet.deserialize(buffer);

    // Deserailize indices and config section
    let reader = new borsh.BinaryReader(Buffer.from(buffer));

    // Read past the header bytes, it would be cleaner to start the buffer at the postHeaderOffset, but could not quickly find right functions
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