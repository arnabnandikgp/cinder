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
      "name": "activateRecovery",
      "docs": [
        "A separate recovery operator activates the immutable statement against actual custody."
      ],
      "discriminator": [
        162,
        213,
        177,
        112,
        118,
        254,
        6,
        67
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
            "recoveryEpoch"
          ]
        },
        {
          "name": "recovery",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "recoveryEpoch",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  99,
                  111,
                  118,
                  101,
                  114,
                  121
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
          "name": "vault",
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
          "name": "epoch",
          "type": "u64"
        },
        {
          "name": "expectedRoot",
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
      "name": "claimRecovery",
      "docs": [
        "Owner-signed full claim; ordinary and recovery payments share lifetime counters."
      ],
      "discriminator": [
        60,
        85,
        49,
        148,
        254,
        62,
        210,
        222
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
            "recoveryEpoch",
            "customer"
          ]
        },
        {
          "name": "recoveryEpoch",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  99,
                  111,
                  118,
                  101,
                  114,
                  121
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
          "name": "owner",
          "writable": true,
          "signer": true,
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
          "name": "claimReceipt",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  99,
                  111,
                  118,
                  101,
                  114,
                  121,
                  95,
                  112,
                  97,
                  105,
                  100
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
          "name": "epoch",
          "type": "u64"
        },
        {
          "name": "claim",
          "type": {
            "defined": {
              "name": "recoveryClaim"
            }
          }
        },
        {
          "name": "proof",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
          }
        }
      ]
    },
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
      "name": "depositEscrow",
      "docs": [
        "Owner-signed exact pending batch; no customer credit or custody-counter change before sweep."
      ],
      "discriminator": [
        226,
        112,
        158,
        176,
        178,
        118,
        153,
        128
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
          },
          "relations": [
            "route",
            "customer"
          ]
        },
        {
          "name": "route",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  101,
                  110,
                  117,
                  101,
                  95,
                  114,
                  111,
                  117,
                  116,
                  101
                ]
              },
              {
                "kind": "account",
                "path": "config"
              }
            ]
          },
          "relations": [
            "escrow"
          ]
        },
        {
          "name": "customer",
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
            "customer",
            "escrow"
          ]
        },
        {
          "name": "escrow",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  101,
                  110,
                  117,
                  101,
                  95,
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "route"
              },
              {
                "kind": "account",
                "path": "owner"
              }
            ]
          }
        },
        {
          "name": "escrowTokens",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119,
                  95,
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
                "path": "escrow"
              }
            ]
          }
        },
        {
          "name": "source",
          "writable": true
        },
        {
          "name": "mint",
          "relations": [
            "config"
          ]
        },
        {
          "name": "batch",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119,
                  95,
                  98,
                  97,
                  116,
                  99,
                  104
                ]
              },
              {
                "kind": "account",
                "path": "escrow"
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
        "Emergency fence only. It does not publish or activate any payable root."
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
      "name": "openVenueEscrow",
      "docs": [
        "Pending owner custody, not native credit or a native subaccount."
      ],
      "discriminator": [
        14,
        225,
        193,
        222,
        128,
        246,
        207,
        205
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
          },
          "relations": [
            "route",
            "customer"
          ]
        },
        {
          "name": "route",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  101,
                  110,
                  117,
                  101,
                  95,
                  114,
                  111,
                  117,
                  116,
                  101
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
          "name": "customer",
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
          "name": "escrow",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  101,
                  110,
                  117,
                  101,
                  95,
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "route"
              },
              {
                "kind": "account",
                "path": "owner"
              }
            ]
          }
        },
        {
          "name": "escrowTokens",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119,
                  95,
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
                "path": "escrow"
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
        }
      ]
    },
    {
      "name": "refundEscrow",
      "docs": [
        "Eligible unswept batches remain owner-refundable after freeze or expiry."
      ],
      "discriminator": [
        107,
        186,
        89,
        99,
        26,
        194,
        23,
        204
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
          },
          "relations": [
            "route"
          ]
        },
        {
          "name": "route",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  101,
                  110,
                  117,
                  101,
                  95,
                  114,
                  111,
                  117,
                  116,
                  101
                ]
              },
              {
                "kind": "account",
                "path": "config"
              }
            ]
          },
          "relations": [
            "escrow"
          ]
        },
        {
          "name": "owner",
          "signer": true,
          "relations": [
            "escrow"
          ]
        },
        {
          "name": "escrow",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  101,
                  110,
                  117,
                  101,
                  95,
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "route"
              },
              {
                "kind": "account",
                "path": "owner"
              }
            ]
          },
          "relations": [
            "batch"
          ]
        },
        {
          "name": "batch",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119,
                  95,
                  98,
                  97,
                  116,
                  99,
                  104
                ]
              },
              {
                "kind": "account",
                "path": "escrow"
              },
              {
                "kind": "account",
                "path": "batch.operation",
                "account": "escrowBatch"
              }
            ]
          }
        },
        {
          "name": "escrowTokens",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119,
                  95,
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
                "path": "escrow"
              }
            ]
          }
        },
        {
          "name": "destination",
          "writable": true
        },
        {
          "name": "mint",
          "relations": [
            "config"
          ]
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
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
      "name": "registerVenueRoute",
      "docs": [
        "Bind this config to one immutable venue and native broker identity."
      ],
      "discriminator": [
        121,
        200,
        139,
        162,
        104,
        180,
        34,
        199
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
          "name": "governance",
          "writable": true,
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "route",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  101,
                  110,
                  117,
                  101,
                  95,
                  114,
                  111,
                  117,
                  116,
                  101
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
          "name": "epoch",
          "type": "u64"
        },
        {
          "name": "venue",
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
    },
    {
      "name": "stageRecovery",
      "docs": [
        "Publish one final, qualified recovery statement after ordinary paths are fenced."
      ],
      "discriminator": [
        246,
        245,
        211,
        14,
        69,
        214,
        14,
        62
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
          "writable": true,
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "recoveryEpoch",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  101,
                  99,
                  111,
                  118,
                  101,
                  114,
                  121
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
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "statement",
          "type": {
            "defined": {
              "name": "recoveryStatement"
            }
          }
        }
      ]
    },
    {
      "name": "sweepEscrow",
      "docs": [
        "Always moves the full stored batch, never the raw escrow balance."
      ],
      "discriminator": [
        199,
        250,
        50,
        208,
        176,
        209,
        42,
        120
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
            "route",
            "customer"
          ]
        },
        {
          "name": "route",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  101,
                  110,
                  117,
                  101,
                  95,
                  114,
                  111,
                  117,
                  116,
                  101
                ]
              },
              {
                "kind": "account",
                "path": "config"
              }
            ]
          },
          "relations": [
            "escrow"
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
          "name": "escrow",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  101,
                  110,
                  117,
                  101,
                  95,
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "route"
              },
              {
                "kind": "account",
                "path": "escrow.owner",
                "account": "venueEscrow"
              }
            ]
          },
          "relations": [
            "batch"
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
                "path": "escrow.owner",
                "account": "venueEscrow"
              }
            ]
          }
        },
        {
          "name": "batch",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119,
                  95,
                  98,
                  97,
                  116,
                  99,
                  104
                ]
              },
              {
                "kind": "account",
                "path": "escrow"
              },
              {
                "kind": "account",
                "path": "batch.operation",
                "account": "escrowBatch"
              }
            ]
          }
        },
        {
          "name": "escrowTokens",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  115,
                  99,
                  114,
                  111,
                  119,
                  95,
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
                "path": "escrow"
              }
            ]
          }
        },
        {
          "name": "brokerTokens",
          "writable": true,
          "relations": [
            "config"
          ]
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
          "name": "expectedSequence",
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
      "name": "escrowBatch",
      "discriminator": [
        53,
        233,
        146,
        81,
        120,
        253,
        158,
        28
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
      "name": "recoveryClaimReceipt",
      "discriminator": [
        118,
        70,
        170,
        64,
        107,
        144,
        226,
        243
      ]
    },
    {
      "name": "recoveryEpoch",
      "discriminator": [
        137,
        13,
        238,
        156,
        58,
        251,
        15,
        95
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
    },
    {
      "name": "venueEscrow",
      "discriminator": [
        64,
        15,
        87,
        181,
        93,
        102,
        177,
        232
      ]
    },
    {
      "name": "venueRoute",
      "discriminator": [
        144,
        188,
        57,
        163,
        80,
        208,
        176,
        186
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
    },
    {
      "code": 6011,
      "name": "recoveryQualification",
      "msg": "Recovery statement is not qualified, complete or final"
    },
    {
      "code": 6012,
      "name": "recoveryProof",
      "msg": "Recovery claim does not match the immutable ordered root"
    },
    {
      "code": 6013,
      "name": "recoveryBacking",
      "msg": "Recovery custody cannot back all remaining claims"
    },
    {
      "code": 6014,
      "name": "recoveryTotal",
      "msg": "Final recovery claim count and declared total disagree"
    },
    {
      "code": 6015,
      "name": "escrowConsumed",
      "msg": "Escrow batch is no longer pending"
    },
    {
      "code": 6016,
      "name": "escrowBacking",
      "msg": "Escrow tokens cannot back all pending owner batches"
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
      "name": "escrowBatch",
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
            "name": "state",
            "type": "u8"
          },
          {
            "name": "escrow",
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
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "epoch",
            "type": "u64"
          },
          {
            "name": "expiresAtSlot",
            "type": "u64"
          },
          {
            "name": "terminalOperation",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "destination",
            "type": "pubkey"
          },
          {
            "name": "sequence",
            "type": "u64"
          },
          {
            "name": "terminalSlot",
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
      "name": "recoveryClaim",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "index",
            "type": "u32"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "paidBase",
            "type": "u64"
          },
          {
            "name": "payoutSequenceBase",
            "type": "u64"
          },
          {
            "name": "claimId",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "salt",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "recoveryClaimReceipt",
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
            "name": "destination",
            "type": "pubkey"
          },
          {
            "name": "epoch",
            "type": "u64"
          },
          {
            "name": "index",
            "type": "u32"
          },
          {
            "name": "claimId",
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
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "paid",
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
      "name": "recoveryEpoch",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "config",
            "type": "pubkey"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "statement",
            "type": {
              "defined": {
                "name": "recoveryStatement"
              }
            }
          },
          {
            "name": "contextHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "remaining",
            "type": "u64"
          },
          {
            "name": "claimedCount",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "recoveryQualification",
      "docs": [
        "Publisher assertions must be independently qualified by the activating operator.",
        "The program checks their shape and rejects explicitly unresolved state; these are",
        "NOT an on-chain proof of external venue fencing, completeness or private accounting."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "unresolvedOperations",
            "type": "u64"
          },
          {
            "name": "outstandingReservations",
            "type": "u64"
          },
          {
            "name": "unresolvedInputs",
            "type": "u64"
          },
          {
            "name": "venueExposureZero",
            "type": "bool"
          },
          {
            "name": "claimsAvailable",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "recoveryStatement",
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
            "name": "root",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "treeSize",
            "type": "u32"
          },
          {
            "name": "total",
            "docs": [
              "Sum of final unpaid claims, already net of ordinary payouts and settled losses."
            ],
            "type": "u64"
          },
          {
            "name": "journalCutoff",
            "type": "u64"
          },
          {
            "name": "journalHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "evidenceHash",
            "docs": [
              "Binds reconciliation, venue fencing/settlement, backing and claim-delivery evidence."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "policyHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "normalPaid",
            "type": "u64"
          },
          {
            "name": "fundingSequence",
            "type": "u64"
          },
          {
            "name": "qualification",
            "type": {
              "defined": {
                "name": "recoveryQualification"
              }
            }
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
    },
    {
      "name": "venueEscrow",
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
            "name": "tokensBump",
            "type": "u8"
          },
          {
            "name": "route",
            "type": "pubkey"
          },
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "pending",
            "docs": [
              "Exact outstanding batches only; donations never increase this obligation."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "venueRoute",
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
            "name": "config",
            "type": "pubkey"
          },
          {
            "name": "venue",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "nativeAccount",
            "type": "pubkey"
          }
        ]
      }
    }
  ]
};
