import ts from "typescript";
import { SomeType } from "../models";
import type { Context } from "./context";
export interface TypeConverter<TNode extends ts.TypeNode = ts.TypeNode, TType extends ts.Type = ts.Type> {
    kind: TNode["kind"][];
    convert(context: Context, node: TNode): SomeType;
    convertType(context: Context, type: TType, node: TNode): SomeType;
}
export declare function loadConverters(): void;
export declare function convertType(context: Context, typeOrNode: ts.Type | ts.TypeNode | undefined): SomeType;
