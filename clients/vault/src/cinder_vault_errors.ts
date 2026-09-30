
export const CinderVaultErrorCode = {
  Domain: 6000,
  Authority: 6001,
  RoleAlias: 6002,
  Mint: 6003,
  TokenAuthority: 6004,
  Mode: 6005,
  Bound: 6006,
  Counter: 6007,
  Expired: 6008,
  Recipient: 6009,
  Arithmetic: 6010,
  RecoveryQualification: 6011,
  RecoveryProof: 6012,
  RecoveryBacking: 6013,
  RecoveryTotal: 6014
};

export type CinderVaultErrorName = keyof typeof CinderVaultErrorCode;
