import { Connection, Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { bootVenue } from "../scripts/venue-boot";
import {
  DUST_CAP,
  loadFundingInterval,
  quoteLotsToUsdc,
} from "../scripts/rise-funding";

const FORK = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8899";

describe("Funding Rise wiring (Surfpool fork)", function () {
  this.timeout(180_000);

  before(function () {
    if (process.env.CINDER_FUNDING !== "1") {
      this.skip();
    }
  });

  it("quoteLotsToUsdc matches adapter toward-zero rules", function () {
    expect(quoteLotsToUsdc(-1000n, 6).toString()).to.equal("-1000");
    expect(quoteLotsToUsdc(5n, 4).toString()).to.equal("500");
    expect(quoteLotsToUsdc(-5_000_000n, 8).toString()).to.equal("-50000");
  });

  it("loads FundingInterval from Rise on a funded fork trader", async function () {
    const connection = new Connection(FORK, "confirmed");
    await connection.getVersion();

    const boot = await bootVenue({
      connection,
      adapter: Keypair.generate(),
      skipWithdraw: true,
    });
    const info = await connection.getAccountInfo(boot.traderPda);
    expect(info).to.not.equal(null);

    const loaded = await loadFundingInterval({
      traderPda: boot.traderPda,
      symbol: "SOL",
      rpcUrl: FORK,
    });
    expect(loaded.isolatedOnly).to.equal(false);
    expect(loaded.fundingIntervalSeconds).to.be.greaterThan(0);
    expect(loaded.fundingPeriodSeconds).to.be.greaterThan(
      loaded.fundingIntervalSeconds
    );
    expect(BigInt(loaded.interval.phoenixCollateral) > 0n).to.equal(true);
    expect(loaded.interval.assetId).to.be.a("number");
    if (loaded.residualLots === "0") {
      expect(BigInt(loaded.dustAbs) <= DUST_CAP).to.equal(true);
    }
  });
});
