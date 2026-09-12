import { Connection } from "@solana/web3.js";
import { expect } from "chai";
import { bootVenue } from "../scripts/venue-boot";

const FORK = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8899";

describe("S5 Rise venue boot (Surfpool fork)", function () {
  this.timeout(180_000);

  before(function () {
    if (process.env.CINDER_S5 !== "1") {
      this.skip();
    }
  });

  it("registers trader 0/0, posts collateral, and pulls or queues", async function () {
    const connection = new Connection(FORK, "confirmed");
    try {
      await connection.getVersion();
    } catch {
      this.skip();
    }

    const out = await bootVenue({ connection });
    const info = await connection.getAccountInfo(out.traderPda);
    expect(info, "trader PDA must exist after RegisterTrader").to.not.equal(
      null
    );
    expect(out.quoteLotCollateralAfterPost > out.quoteLotCollateralBefore).to.equal(
      true
    );

    if (out.withdrawQueued) {
      expect(out.withdrawQueued).to.equal(true);
    } else {
      expect(out.quoteLotCollateralAfterPull).to.not.equal(null);
      expect(
        out.quoteLotCollateralAfterPull! < out.quoteLotCollateralAfterPost
      ).to.equal(true);
    }
  });
});
