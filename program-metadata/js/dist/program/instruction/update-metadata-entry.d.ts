import { Struct } from "../util/borsh-struct";
export declare class UpdateMetadataEntryInstruction extends Struct {
    value: string;
    instruction: number;
    constructor(value: string);
}
