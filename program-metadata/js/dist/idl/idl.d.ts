export declare type Idl = {
    version: string;
    name: string;
    instructions: IdlInstruction[];
    state?: IdlState;
    accounts?: IdlTypeDef[];
    types?: IdlTypeDef[];
    events?: IdlEvent[];
    errors?: IdlErrorCode[];
};
export declare type IdlEvent = {
    name: string;
    fields: IdlEventField[];
};
export declare type IdlEventField = {
    name: string;
    type: IdlType;
    index: boolean;
};
export declare type IdlInstruction = {
    name: string;
    accounts: IdlAccountItem[];
    args: IdlField[];
};
export declare type IdlState = {
    struct: IdlTypeDef;
    methods: IdlStateMethod[];
};
export declare type IdlStateMethod = IdlInstruction;
export declare type IdlAccountItem = IdlAccount | IdlAccounts;
export declare type IdlAccount = {
    name: string;
    isMut: boolean;
    isSigner: boolean;
};
export declare type IdlAccounts = {
    name: string;
    accounts: IdlAccountItem[];
};
export declare type IdlField = {
    name: string;
    type: IdlType;
};
export declare type IdlTypeDef = {
    name: string;
    type: IdlTypeDefTy;
};
declare type IdlTypeDefTy = {
    kind: "struct" | "enum";
    fields?: IdlTypeDefStruct;
    variants?: IdlEnumVariant[];
};
declare type IdlTypeDefStruct = Array<IdlField>;
export declare type IdlType = "bool" | "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "u128" | "i128" | "bytes" | "string" | "publicKey" | IdlTypeVec | IdlTypeOption | IdlTypeDefined;
export declare type IdlTypeVec = {
    vec: IdlType;
};
export declare type IdlTypeOption = {
    option: IdlType;
};
export declare type IdlTypeDefined = {
    defined: string;
};
export declare type IdlEnumVariant = {
    name: string;
    fields?: IdlEnumFields;
};
declare type IdlEnumFields = IdlEnumFieldsNamed | IdlEnumFieldsTuple;
declare type IdlEnumFieldsNamed = IdlField[];
declare type IdlEnumFieldsTuple = IdlType[];
declare type IdlErrorCode = {
    code: number;
    name: string;
    msg?: string;
};
export {};
