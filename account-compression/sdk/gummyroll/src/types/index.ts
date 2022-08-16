// TODO(sorend): do we want to export the Anchor type in the SDK? We can modify all imports from contracts/target if so, but then we might need to custom hook into anchor build/test to keep this up to date.
export type Gummyroll = {
  "version": "0.1.0",
  "name": "gummyroll",
  "instructions": [
    {
      "name": "initEmptyGummyroll",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "candyWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxDepth",
          "type": "u32"
        },
        {
          "name": "maxBufferSize",
          "type": "u32"
        }
      ]
    },
    {
      "name": "initGummyrollWithRoot",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "candyWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxDepth",
          "type": "u32"
        },
        {
          "name": "maxBufferSize",
          "type": "u32"
        },
        {
          "name": "root",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "leaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "index",
          "type": "u32"
        },
        {
          "name": "changelogDbUri",
          "type": "string"
        },
        {
          "name": "metadataDbUri",
          "type": "string"
        }
      ]
    },
    {
      "name": "replaceLeaf",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "candyWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "root",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "previousLeaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "newLeaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "index",
          "type": "u32"
        }
      ]
    },
    {
      "name": "transferAuthority",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "newAuthority",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "verifyLeaf",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "root",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "leaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "index",
          "type": "u32"
        }
      ]
    },
    {
      "name": "append",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "candyWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "leaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    },
    {
      "name": "insertOrAppend",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "candyWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "root",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "leaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "index",
          "type": "u32"
        }
      ]
    }
  ],
  "types": [
    {
      "name": "PathNode",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "node",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "index",
            "type": "u32"
          }
        ]
      }
    }
  ],
  "events": [
    {
      "name": "NewLeafEvent",
      "fields": [
        {
          "name": "id",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "leaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          },
          "index": false
        }
      ]
    },
    {
      "name": "ChangeLogEvent",
      "fields": [
        {
          "name": "id",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "path",
          "type": {
            "vec": {
              "defined": "PathNode"
            }
          },
          "index": false
        },
        {
          "name": "seq",
          "type": "u64",
          "index": false
        },
        {
          "name": "index",
          "type": "u32",
          "index": false
        }
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "IncorrectLeafLength",
      "msg": "Incorrect leaf length. Expected vec of 32 bytes"
    },
    {
      "code": 6001,
      "name": "ConcurrentMerkleTreeError",
      "msg": "Concurrent merkle tree error"
    },
    {
      "code": 6002,
      "name": "ZeroCopyError",
      "msg": "Issue zero copying concurrent merkle tree data"
    },
    {
      "code": 6003,
      "name": "MerkleRollConstantsError",
      "msg": "An unsupported max depth or max buffer size constant was provided"
    },
    {
      "code": 6004,
      "name": "CanopyLengthMismatch",
      "msg": "Expected a different byte length for the merkle roll canopy"
    }
  ]
};

export const IDL: Gummyroll = {
  "version": "0.1.0",
  "name": "gummyroll",
  "instructions": [
    {
      "name": "initEmptyGummyroll",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "candyWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxDepth",
          "type": "u32"
        },
        {
          "name": "maxBufferSize",
          "type": "u32"
        }
      ]
    },
    {
      "name": "initGummyrollWithRoot",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "candyWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxDepth",
          "type": "u32"
        },
        {
          "name": "maxBufferSize",
          "type": "u32"
        },
        {
          "name": "root",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "leaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "index",
          "type": "u32"
        },
        {
          "name": "changelogDbUri",
          "type": "string"
        },
        {
          "name": "metadataDbUri",
          "type": "string"
        }
      ]
    },
    {
      "name": "replaceLeaf",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "candyWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "root",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "previousLeaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "newLeaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "index",
          "type": "u32"
        }
      ]
    },
    {
      "name": "transferAuthority",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "newAuthority",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "verifyLeaf",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "root",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "leaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "index",
          "type": "u32"
        }
      ]
    },
    {
      "name": "append",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "candyWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "leaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    },
    {
      "name": "insertOrAppend",
      "accounts": [
        {
          "name": "merkleRoll",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "candyWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "root",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "leaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "index",
          "type": "u32"
        }
      ]
    }
  ],
  "types": [
    {
      "name": "PathNode",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "node",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "index",
            "type": "u32"
          }
        ]
      }
    }
  ],
  "events": [
    {
      "name": "NewLeafEvent",
      "fields": [
        {
          "name": "id",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "leaf",
          "type": {
            "array": [
              "u8",
              32
            ]
          },
          "index": false
        }
      ]
    },
    {
      "name": "ChangeLogEvent",
      "fields": [
        {
          "name": "id",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "path",
          "type": {
            "vec": {
              "defined": "PathNode"
            }
          },
          "index": false
        },
        {
          "name": "seq",
          "type": "u64",
          "index": false
        },
        {
          "name": "index",
          "type": "u32",
          "index": false
        }
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "IncorrectLeafLength",
      "msg": "Incorrect leaf length. Expected vec of 32 bytes"
    },
    {
      "code": 6001,
      "name": "ConcurrentMerkleTreeError",
      "msg": "Concurrent merkle tree error"
    },
    {
      "code": 6002,
      "name": "ZeroCopyError",
      "msg": "Issue zero copying concurrent merkle tree data"
    },
    {
      "code": 6003,
      "name": "MerkleRollConstantsError",
      "msg": "An unsupported max depth or max buffer size constant was provided"
    },
    {
      "code": 6004,
      "name": "CanopyLengthMismatch",
      "msg": "Expected a different byte length for the merkle roll canopy"
    }
  ]
};
