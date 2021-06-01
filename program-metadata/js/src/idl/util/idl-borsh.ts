import {
  IdlEnumVariant,
  IdlError,
  IdlField,
  IdlType,
  IdlTypeDef,
} from "../idl";
import * as borsh from "@project-serum/borsh";
import camelCase from "camelcase";
import { Layout } from "buffer-layout";

type IdlFieldWithoutName = {
  type: IdlType;
};

export function fieldLayout(
  field: IdlField | IdlFieldWithoutName,
  types?: IdlTypeDef[]
): Layout {
  let fieldName;

  if ("name" in field) {
    fieldName = camelCase(field.name);
  }

  switch (field.type) {
    case "bool": {
      return borsh.bool(fieldName);
    }
    case "u8": {
      return borsh.u8(fieldName);
    }
    case "i8": {
      return borsh.i8(fieldName);
    }
    case "u16": {
      return borsh.u16(fieldName);
    }
    case "i16": {
      return borsh.i16(fieldName);
    }
    case "u32": {
      return borsh.u32(fieldName);
    }
    case "i32": {
      return borsh.i32(fieldName);
    }
    case "u64": {
      return borsh.u64(fieldName);
    }
    case "i64": {
      return borsh.i64(fieldName);
    }
    case "u128": {
      return borsh.u128(fieldName);
    }
    case "i128": {
      return borsh.i128(fieldName);
    }
    case "bytes": {
      return borsh.vecU8(fieldName);
    }
    case "string": {
      return borsh.str(fieldName);
    }
    case "publicKey": {
      return borsh.publicKey(fieldName);
    }
    default: {
      // @ts-ignore
      if (field.type.vec) {
        return borsh.vec(
          fieldLayout(
            {
              name: undefined,
              // @ts-ignore
              type: field.type.vec,
            },
            types
          ),
          fieldName
        );
        // @ts-ignore
      } else if (field.type.option) {
        return borsh.option(
          fieldLayout(
            {
              name: undefined,
              // @ts-ignore
              type: field.type.option,
            },
            types
          ),
          fieldName
        );
        // @ts-ignore
      } else if (field.type.defined) {
        // User defined type.
        if (types === undefined) {
          throw new IdlError("User defined types not provided");
        }
        // @ts-ignore
        const filtered = types.filter((t) => t.name === field.type.defined);
        if (filtered.length !== 1) {
          throw new IdlError(`Type not found: ${JSON.stringify(field)}`);
        }
        return typeDefLayout(filtered[0], types, fieldName);
        // @ts-ignore
      } else if (field.type.array) {
        // @ts-ignore
        let arrayTy = field.type.array[0];
        // @ts-ignore
        let arrayLen = field.type.array[1];
        let innerLayout = fieldLayout(
          {
            name: undefined,
            type: arrayTy,
          },
          types
        );
        return borsh.array(innerLayout, arrayLen, fieldName);
      } else {
        throw new Error(`Not yet implemented: ${field}`);
      }
    }
  }
}

export function typeDefLayout(
  typeDef: IdlTypeDef,
  types: IdlTypeDef[],
  name?: string
): Layout {
  if (typeDef.type.kind === "struct") {
    const fieldLayouts = (typeDef.type.fields || []).map((field) => {
      const x = fieldLayout(field, types);
      return x;
    });
    return borsh.struct(fieldLayouts, name);
  } else if (typeDef.type.kind === "enum") {
    let variants = (typeDef.type.variants || []).map(
      (variant: IdlEnumVariant) => {
        const name = camelCase(variant.name);
        if (variant.fields === undefined) {
          return borsh.struct([], name);
        }
        // @ts-ignore
        const fieldLayouts = variant.fields.map((f: IdlField | IdlType) => {
          // @ts-ignore
          if (f.name === undefined) {
            throw new Error("Tuple enum variants not yet implemented.");
          }
          // @ts-ignore
          return IdlCoder.fieldLayout(f, types);
        });
        return borsh.struct(fieldLayouts, name);
      }
    );

    if (name !== undefined) {
      // Buffer-layout lib requires the name to be null (on construction)
      // when used as a field.
      return borsh.rustEnum(variants).replicate(name);
    }

    return borsh.rustEnum(variants, name);
  } else {
    throw new Error(`Unknown type kint: ${typeDef}`);
  }
}
