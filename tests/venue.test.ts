import { Connection, Keypair } from "@solana/web3.js";
import { expect } from "chai";
import {
  bootVenue,
  getReferralActivationPermission,
} from "../scripts/venue-boot";

const FORK = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8899";

describe("Phoenix referral activation permission", function () {
  const livePermission = {
    trader_onboarder: "11111111111111111111111111111111",
    risk_authority: "SysvarRent111111111111111111111111111111111",
    permission_account: "SysvarC1ock11111111111111111111111111111111",
  };

  it("uses the API response when the endpoint succeeds", async function () {
    const result = await getReferralActivationPermission(
      { getReferralActivationPermission: async () => livePermission },
      () => undefined
    );

    expect(result).to.deep.equal(livePermission);
  });

  it("uses the pinned response when the endpoint returns HTTP 429", async function () {
    const result = await getReferralActivationPermission(
      {
        getReferralActivationPermission: async () => {
          throw Object.assign(new Error("rate limited"), { status: 429 });
        },
      },
      () => undefined
    );

    expect(result).to.deep.equal({
      trader_onboarder: "EzkM8YbCkBLaCqX2cdxtMyxfTLpKui3mWQWnhe5w2P4Z",
      risk_authority: "8wTcJdg4Xw3wnvmBu7Sokn3c2UnnHUhuB5g9sfeCMidR",
      permission_account: "6UCimUpRW2CGh3ESJm3S59c4WcCaVLYcpPPbbrfX2nzr",
    });
  });

  it("does not hide non-rate-limit failures", async function () {
    const failure = Object.assign(new Error("service unavailable"), {
      status: 503,
    });

    try {
      await getReferralActivationPermission(
        {
          getReferralActivationPermission: async () => {
            throw failure;
          },
        },
        () => undefined
      );
      expect.fail("expected the request to fail");
    } catch (error) {
      expect(error).to.equal(failure);
    }
  });
});

describe("Phoenix venue boot (Surfpool fork)", function () {
  this.timeout(180_000);

  before(function () {
    if (process.env.CINDER_VENUE !== "1") {
      this.skip();
    }
  });

  it("registers trader 0/0, posts collateral, and pulls or queues", async function () {
    const connection = new Connection(FORK, "confirmed");
    await connection.getVersion();

    const out = await bootVenue({
      connection,
      adapter: Keypair.generate(),
    });
    const info = await connection.getAccountInfo(out.traderPda);
    expect(info, "trader PDA must exist after RegisterTrader").to.not.equal(
      null
    );
    expect(out.quoteLotCollateralBefore).to.equal(0n);
    expect(out.quoteLotCollateralAfterPost).to.equal(25_000_000n);

    if (out.withdrawQueued) {
      expect(out.quoteLotCollateralAfterPull).to.equal(
        out.quoteLotCollateralAfterPost
      );
    } else {
      expect(out.quoteLotCollateralAfterPull).to.equal(0n);
    }
  });
});
