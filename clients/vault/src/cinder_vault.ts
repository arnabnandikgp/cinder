/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/cinder_vault.json`.
 */
export type CinderVault = {
  "address": "FMu4FVg4kWABgsPg8Q5Q8Z3wieTtVgtG2gapCSdGT7RX",
  "metadata": {
    "name": "cinderVault",
    "version": "0.1.0",
    "spec": "0.1.0"
  },
  "instructions": [
    {
      "name": "deposit",
      "docs": [
        "An immutable receipt is created only in the same transaction as actual token arrival."
      ],
      "discriminator": [
        242,
        35,
        198,
        137,
        82,
        225,
        242,
        182
      ],
      "accounts": [
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  105,
                  110,
                  100,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config.domain",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.pool",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.mint",
                "account": "vaultConfig"
              }
            ]
          },
          "relations": [
            "customer"
          ]
        },
        {
          "name": "customer",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  117,
                  115,
                  116,
                  111,
                  109,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "config"
              },
              {
                "kind": "account",
                "path": "owner"
              }
            ]
          }
        },
        {
          "name": "owner",
          "writable": true,
          "signer": true,
          "relations": [
            "customer"
          ]
        },
        {
          "name": "source",
          "writable": true
        },
        {
          "name": "vault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110,
                  115
                ]
              },
              {
                "kind": "account",
                "path": "config"
              }
            ]
          }
        },
        {
          "name": "mint",
          "relations": [
            "config"
          ]
        },
        {
          "name": "receipt",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  112,
                  111,
                  115,
                  105,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config"
              },
              {
                "kind": "account",
                "path": "owner"
              },
              {
                "kind": "arg",
                "path": "auth.operation"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "auth",
          "type": {
            "defined": {
              "name": "movement"
            }
          }
        },
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "freeze",
      "docs": [
        "Emergency fence only. No heartbeat, root publication, recovery payout or resume path."
      ],
      "discriminator": [
        255,
        91,
        207,
        84,
        251,
        194,
        254,
        63
      ],
      "accounts": [
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  105,
                  110,
                  100,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config.domain",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.pool",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.mint",
                "account": "vaultConfig"
              }
            ]
          }
        },
        {
          "name": "recovery",
          "signer": true,
          "relations": [
            "config"
          ]
        }
      ],
      "args": [
        {
          "name": "domain",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "expectedEpoch",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initialize",
      "docs": [
        "Only this executable's actual loader upgrade authority can bootstrap custody."
      ],
      "discriminator": [
        175,
        175,
        109,
        31,
        13,
        152,
        155,
        237
      ],
      "accounts": [
        {
          "name": "governance",
          "writable": true,
          "signer": true
        },
        {
          "name": "funds",
          "signer": true
        },
        {
          "name": "recovery",
          "signer": true
        },
        {
          "name": "program",
          "address": "FMu4FVg4kWABgsPg8Q5Q8Z3wieTtVgtG2gapCSdGT7RX"
        },
        {
          "name": "programData"
        },
        {
          "name": "mint"
        },
        {
          "name": "brokerTokens"
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  105,
                  110,
                  100,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "arg",
                "path": "domain"
              },
              {
                "kind": "arg",
                "path": "pool"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "vault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110,
                  115
                ]
              },
              {
                "kind": "account",
                "path": "config"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "domain",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "pool",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "fundingCap",
          "type": "u64"
        },
        {
          "name": "payoutCap",
          "type": "u64"
        }
      ]
    },
    {
      "name": "normalPayout",
      "docs": [
        "Funds authority approves a ledger-qualified payout; deposits are NOT its entitlement cap.",
        "Customer/asset paid totals and sequence survive authority rotation and later recovery."
      ],
      "discriminator": [
        250,
        168,
        237,
        93,
        116,
        74,
        152,
        237
      ],
      "accounts": [
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  105,
                  110,
                  100,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config.domain",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.pool",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.mint",
                "account": "vaultConfig"
              }
            ]
          },
          "relations": [
            "customer"
          ]
        },
        {
          "name": "funds",
          "writable": true,
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "owner",
          "relations": [
            "customer"
          ]
        },
        {
          "name": "customer",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  117,
                  115,
                  116,
                  111,
                  109,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "config"
              },
              {
                "kind": "account",
                "path": "owner"
              }
            ]
          }
        },
        {
          "name": "destination",
          "writable": true
        },
        {
          "name": "vault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110,
                  115
                ]
              },
              {
                "kind": "account",
                "path": "config"
              }
            ]
          }
        },
        {
          "name": "mint",
          "relations": [
            "config"
          ]
        },
        {
          "name": "receipt",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  99,
                  101,
                  105,
                  112,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config"
              },
              {
                "kind": "arg",
                "path": "auth.operation"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "auth",
          "type": {
            "defined": {
              "name": "movement"
            }
          }
        },
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "expectedPaid",
          "type": "u64"
        },
        {
          "name": "expectedSequence",
          "type": "u64"
        }
      ]
    },
    {
      "name": "registerCustomer",
      "docs": [
        "Public wallet attribution only; never stores current trading equity or positions."
      ],
      "discriminator": [
        211,
        203,
        193,
        164,
        198,
        246,
        27,
        223
      ],
      "accounts": [
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  105,
                  110,
                  100,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config.domain",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.pool",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.mint",
                "account": "vaultConfig"
              }
            ]
          }
        },
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "customer",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  117,
                  115,
                  116,
                  111,
                  109,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "config"
              },
              {
                "kind": "account",
                "path": "owner"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "domain",
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
      "name": "releaseFunding",
      "docs": [
        "Releases working collateral to one immutable, explicitly allowlisted broker route.",
        "Epoch budget is gross cumulative release; returns do not silently replenish it."
      ],
      "discriminator": [
        213,
        91,
        216,
        103,
        251,
        46,
        51,
        145
      ],
      "accounts": [
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  105,
                  110,
                  100,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config.domain",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.pool",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.mint",
                "account": "vaultConfig"
              }
            ]
          }
        },
        {
          "name": "funds",
          "writable": true,
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "brokerTokens",
          "writable": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110,
                  115
                ]
              },
              {
                "kind": "account",
                "path": "config"
              }
            ]
          }
        },
        {
          "name": "mint",
          "relations": [
            "config"
          ]
        },
        {
          "name": "receipt",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  99,
                  101,
                  105,
                  112,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config"
              },
              {
                "kind": "arg",
                "path": "auth.operation"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "auth",
          "type": {
            "defined": {
              "name": "movement"
            }
          }
        },
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "expectedSequence",
          "type": "u64"
        }
      ]
    },
    {
      "name": "returnFunding",
      "docs": [
        "Returning custody does not credit a customer again. Permitted while frozen too."
      ],
      "discriminator": [
        160,
        22,
        207,
        81,
        205,
        5,
        193,
        157
      ],
      "accounts": [
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  105,
                  110,
                  100,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config.domain",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.pool",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.mint",
                "account": "vaultConfig"
              }
            ]
          }
        },
        {
          "name": "broker",
          "writable": true,
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "source",
          "writable": true
        },
        {
          "name": "vault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110,
                  115
                ]
              },
              {
                "kind": "account",
                "path": "config"
              }
            ]
          }
        },
        {
          "name": "mint",
          "relations": [
            "config"
          ]
        },
        {
          "name": "receipt",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  99,
                  101,
                  105,
                  112,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config"
              },
              {
                "kind": "arg",
                "path": "auth.operation"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "auth",
          "type": {
            "defined": {
              "name": "movement"
            }
          }
        },
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "rotate",
      "docs": [
        "Atomic role handoff, signed by the old and new governance. Epoch invalidates old wires.",
        "This changes no historical receipt or paid counter; it cannot revive a frozen pool."
      ],
      "discriminator": [
        217,
        166,
        249,
        176,
        3,
        217,
        200,
        73
      ],
      "accounts": [
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  105,
                  110,
                  100,
                  101,
                  114,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "config.domain",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.pool",
                "account": "vaultConfig"
              },
              {
                "kind": "account",
                "path": "config.mint",
                "account": "vaultConfig"
              }
            ]
          }
        },
        {
          "name": "governance",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "newGovernance",
          "signer": true
        },
        {
          "name": "newFunds",
          "signer": true
        },
        {
          "name": "newRecovery",
          "signer": true
        }
      ],
      "args": [
        {
          "name": "domain",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "expectedEpoch",
          "type": "u64"
        },
        {
          "name": "fundingCap",
          "type": "u64"
        },
        {
          "name": "payoutCap",
          "type": "u64"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "customerCounter",
      "discriminator": [
        210,
        46,
        219,
        50,
        118,
        203,
        216,
        24
      ]
    },
    {
      "name": "movementReceipt",
      "discriminator": [
        55,
        129,
        93,
        192,
        203,
        162,
        232,
        52
      ]
    },
    {
      "name": "vaultConfig",
      "discriminator": [
        99,
        86,
        43,
        216,
        184,
        102,
        119,
        77
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "domain",
      "msg": "Wrong deployment domain, authority epoch or empty identity"
    },
    {
      "code": 6001,
      "name": "authority",
      "msg": "Unauthorized account or non-signing role"
    },
    {
      "code": 6002,
      "name": "roleAlias",
      "msg": "Custody roles must be distinct"
    },
    {
      "code": 6003,
      "name": "mint",
      "msg": "Unsupported mint precision"
    },
    {
      "code": 6004,
      "name": "tokenAuthority",
      "msg": "Delegated or externally closeable token account"
    },
    {
      "code": 6005,
      "name": "mode",
      "msg": "Invalid custody mode"
    },
    {
      "code": 6006,
      "name": "bound",
      "msg": "Custody amount exceeds its bound"
    },
    {
      "code": 6007,
      "name": "counter",
      "msg": "Stale funding or payout counter"
    },
    {
      "code": 6008,
      "name": "expired",
      "msg": "Movement authorization expired"
    },
    {
      "code": 6009,
      "name": "recipient",
      "msg": "Invalid recipient"
    },
    {
      "code": 6010,
      "name": "arithmetic",
      "msg": "Integer overflow"
    }
  ],
  "types": [
    {
      "name": "customerCounter",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "config",
            "type": "pubkey"
          },
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "deposited",
            "type": "u64"
          },
          {
            "name": "paid",
            "docs": [
              "Shared normal/recovery total, never reset on authority or recovery-root changes."
            ],
            "type": "u64"
          },
          {
            "name": "payoutSequence",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "movement",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "domain",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "epoch",
            "type": "u64"
          },
          {
            "name": "operation",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "expiresAtSlot",
            "docs": [
              "Latest acceptable execution slot, not evidence of external finality."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "movementReceipt",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "config",
            "type": "pubkey"
          },
          {
            "name": "operation",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "epoch",
            "type": "u64"
          },
          {
            "name": "kind",
            "type": "u8"
          },
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "destination",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "sequence",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "vaultConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "schema",
            "type": "u8"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "vaultBump",
            "type": "u8"
          },
          {
            "name": "decimals",
            "type": "u8"
          },
          {
            "name": "domain",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "pool",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "governance",
            "type": "pubkey"
          },
          {
            "name": "funds",
            "type": "pubkey"
          },
          {
            "name": "recovery",
            "type": "pubkey"
          },
          {
            "name": "broker",
            "type": "pubkey"
          },
          {
            "name": "brokerTokens",
            "type": "pubkey"
          },
          {
            "name": "epoch",
            "type": "u64"
          },
          {
            "name": "mode",
            "type": "u8"
          },
          {
            "name": "fundingCap",
            "type": "u64"
          },
          {
            "name": "payoutCap",
            "type": "u64"
          },
          {
            "name": "epochReleased",
            "type": "u64"
          },
          {
            "name": "fundingSequence",
            "type": "u64"
          },
          {
            "name": "deposited",
            "type": "u64"
          },
          {
            "name": "returned",
            "type": "u64"
          },
          {
            "name": "paid",
            "type": "u64"
          }
        ]
      }
    }
  ]
};
