import { createHash } from "crypto";
import { chmodSync, mkdirSync, readFileSync, writeFileSync } from "fs";
import { join, resolve } from "path";
import { Keypair, PublicKey } from "@solana/web3.js";

const LEDGER_PROGRAM = new PublicKey(
  "h3Bw2xjj69JssRkaxr8Jxh6TtamvrjSxASXbfLeWyPg"
);
const VAULT_PROGRAM = new PublicKey(
  "9zhBFVgk13gnYT6iVuKPGfQiAvVfr6cYQq2bY2QUzXmg"
);
const ER_VALIDATOR = new PublicKey(
  "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
);

function discriminator(name: string): Buffer {
  return createHash("sha256").update(`account:${name}`).digest().subarray(0, 8);
}

function u8(value: number): Buffer {
  return Buffer.from([value]);
}

function u16(value: number): Buffer {
  const out = Buffer.alloc(2);
  out.writeUInt16LE(value);
  return out;
}

function u32(value: number): Buffer {
  const out = Buffer.alloc(4);
  out.writeUInt32LE(value);
  return out;
}

function u64(value: bigint | number): Buffer {
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(BigInt(value));
  return out;
}

function i64(value: bigint | number): Buffer {
  const out = Buffer.alloc(8);
  out.writeBigInt64LE(BigInt(value));
  return out;
}

function pubkey(value: PublicKey): Buffer {
  return value.toBuffer();
}

function accountData(name: string, fields: Buffer[]): Buffer {
  return Buffer.concat([discriminator(name), ...fields]);
}

function position(
  assetId = 0,
  lots = 0,
  entryQuoteLots = 0,
  unsettledFunding = 0,
  reservedIm = 0
): Buffer {
  return Buffer.concat([
    u16(assetId),
    i64(lots),
    i64(entryQuoteLots),
    i64(unsettledFunding),
    u64(reservedIm),
  ]);
}

function openOid(): Buffer {
  return Buffer.concat([Buffer.alloc(16), u16(0), i64(0), u8(0)]);
}

function residual(assetId = 0, lots = 0): Buffer {
  return Buffer.concat([u16(assetId), i64(lots)]);
}

function writeAccount(
  directory: string,
  name: string,
  address: PublicKey,
  owner: PublicKey,
  data: Buffer
) {
  const body = {
    pubkey: address.toBase58(),
    account: {
      lamports: 10_000_000_000,
      data: [data.toString("base64"), "base64"],
      owner: owner.toBase58(),
      executable: false,
      rentEpoch: 0,
      space: data.length,
    },
  };
  writeFileSync(join(directory, `${name}.json`), `${JSON.stringify(body, null, 2)}\n`);
}

function main() {
  const output = process.argv[2];
  const walletPath = process.env.ANCHOR_WALLET;
  if (!output || !walletPath) {
    throw new Error("usage: ANCHOR_WALLET=<keypair> create-schema-fixture.ts <output-dir>");
  }

  const directory = resolve(output);
  const accountsDirectory = join(directory, "accounts");
  mkdirSync(accountsDirectory, { recursive: true });

  const adapter = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(readFileSync(walletPath, "utf8")))
  );
  const user = Keypair.generate();
  const userKeypairPath = join(directory, "user.json");
  writeFileSync(userKeypairPath, `${JSON.stringify(Array.from(user.secretKey))}\n`);
  chmodSync(userKeypairPath, 0o600);

  const [config, configBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    VAULT_PROGRAM
  );
  const [reserveRoot] = PublicKey.findProgramAddressSync(
    [Buffer.from("reserve")],
    VAULT_PROGRAM
  );
  const [book, bookBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("book")],
    LEDGER_PROGRAM
  );
  const [userLedger, userBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("user"), user.publicKey.toBuffer()],
    LEDGER_PROGRAM
  );

  const empty = PublicKey.default;
  const configData = accountData("Config", [
    pubkey(adapter.publicKey),
    pubkey(adapter.publicKey),
    pubkey(empty),
    pubkey(empty),
    pubkey(empty),
    pubkey(empty),
    pubkey(ER_VALIDATOR),
    u8(1),
    u16(12_500),
    u16(12_500),
    u16(10),
    u16(2_000),
    u64(50_000_000),
    u8(1),
    u16(1),
    ...Array.from({ length: 31 }, () => u16(0)),
    u8(configBump),
    u8(0),
  ]);

  const reserveData = accountData("ReserveRoot", [
    u64(4),
    Buffer.alloc(32, 3),
    u32(1),
    u64(98_750_000),
    u64(1_250_000),
    Buffer.alloc(32, 4),
    u64(55),
  ]);

  const positions = [
    position(1, 10, 10_000_000, 0, 1_250_000),
    ...Array.from({ length: 15 }, () => position()),
  ];
  const userData = accountData("UserLedger", [
    pubkey(user.publicKey),
    u64(98_750_000),
    u64(1_250_000),
    u64(0),
    u8(0),
    u64(7),
    u64(3),
    u8(1),
    ...positions,
    ...Array.from({ length: 8 }, () => openOid()),
    u8(userBump),
  ]);

  const bookData = accountData("Book", [
    u8(1),
    residual(1, 10),
    ...Array.from({ length: 31 }, () => residual()),
    u64(50_000_000),
    u64(44),
    u8(1),
    u8(1),
    u64(3),
    u64(1234),
    u8(bookBump),
  ]);

  if (configData.length !== 316 || reserveData.length !== 108) {
    throw new Error("vault legacy fixture layout drifted");
  }
  if (userData.length !== 843 || bookData.length !== 364) {
    throw new Error("ledger legacy fixture layout drifted");
  }

  writeAccount(accountsDirectory, "config", config, VAULT_PROGRAM, configData);
  writeAccount(accountsDirectory, "reserve", reserveRoot, VAULT_PROGRAM, reserveData);
  writeAccount(accountsDirectory, "book", book, LEDGER_PROGRAM, bookData);
  writeAccount(accountsDirectory, "user-ledger", userLedger, LEDGER_PROGRAM, userData);

  writeFileSync(
    join(directory, "manifest.json"),
    `${JSON.stringify(
      {
        adapter: adapter.publicKey.toBase58(),
        user: user.publicKey.toBase58(),
        userKeypairPath,
        config: config.toBase58(),
        reserveRoot: reserveRoot.toBase58(),
        book: book.toBase58(),
        userLedger: userLedger.toBase58(),
      },
      null,
      2
    )}\n`
  );
}

main();
