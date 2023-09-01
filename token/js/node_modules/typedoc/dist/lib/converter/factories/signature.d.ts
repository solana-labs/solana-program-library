import ts from "typescript";
import { ParameterReflection, ReflectionKind, SignatureReflection, TypeParameterReflection } from "../../models";
import type { Context } from "../context";
export declare function createSignature(context: Context, kind: ReflectionKind.CallSignature | ReflectionKind.ConstructorSignature | ReflectionKind.GetSignature | ReflectionKind.SetSignature, signature: ts.Signature, symbol: ts.Symbol | undefined, declaration?: ts.SignatureDeclaration | ts.JSDocSignature): void;
export declare function convertParameterNodes(context: Context, sigRef: SignatureReflection, parameters: readonly (ts.JSDocParameterTag | ts.ParameterDeclaration)[]): ParameterReflection[];
export declare function convertTypeParameterNodes(context: Context, parameters: readonly ts.TypeParameterDeclaration[] | undefined): TypeParameterReflection[] | undefined;
export declare function createTypeParamReflection(param: ts.TypeParameterDeclaration, context: Context): TypeParameterReflection;
