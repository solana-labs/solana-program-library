import { IdlField, IdlType, IdlTypeDef } from "../idl";
import { Layout } from "buffer-layout";
declare type IdlFieldWithoutName = {
    type: IdlType;
};
export declare function fieldLayout(field: IdlField | IdlFieldWithoutName, types?: IdlTypeDef[]): Layout;
export declare function typeDefLayout(typeDef: IdlTypeDef, types: IdlTypeDef[], name?: string): Layout;
export {};
