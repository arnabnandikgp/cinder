/**
 * Process-edge Rise/Hawkeye wiring for FundingInterval (`docs/08` §7).
 * Reads on-chain trader state from a local fork; market metadata from the public API.
 */
import { Connection, PublicKey } from "@solana/web3.js";

const FORK = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8899";
const API = process.env.PHOENIX_API_URL || "https://perp-api.phoenix.trade";
const MAINNET_GENESIS = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";
const USDC_DECIMALS = 6;
const DUST_CAP = 1000n;

export type FundingIntervalJson = {
  assetId: number;
  usdcPerLot: string;
  poolUnsettledUsdc: string;
  phoenixCollateral: string;
};

function assertLocalRpc(url: string) {
  const parsed = new URL(url);
  if (parsed.hostname !== "127.0.0.1" && parsed.hostname !== "localhost") {
    throw new Error(`refusing non-local RPC ${url}`);
  }
}

function pow10(n: number): bigint {
  let x = 1n;
  for (let i = 0; i < n; i++) x *= 10n;
  return x;
}

/** Same rules as adapter `quote_lots_to_usdc` (toward zero). */
export function quoteLotsToUsdc(
  quoteLots: bigint,
  quoteDecimals: number
): bigint {
  if (quoteDecimals === USDC_DECIMALS) return quoteLots;
  if (quoteDecimals < USDC_DECIMALS) {
    return quoteLots * pow10(USDC_DECIMALS - quoteDecimals);
  }
  return quoteLots / pow10(quoteDecimals - USDC_DECIMALS);
}

function toBigInt(v: { toString(): string } | number | string | bigint): bigint {
  return BigInt(typeof v === "number" ? Math.trunc(v).toString() : v.toString());
}

export async function loadFundingInterval(opts: {
  traderPda: PublicKey;
  symbol?: string;
  rpcUrl?: string;
}): Promise<{
  interval: FundingIntervalJson;
  isolatedOnly: boolean;
  fundingIntervalSeconds: number;
  fundingPeriodSeconds: number;
  residualLots: string;
  dustAbs: string;
}> {
  const rpcUrl = opts.rpcUrl ?? FORK;
  assertLocalRpc(rpcUrl);
  const connection = new Connection(rpcUrl, "confirmed");
  const genesis = await connection.getGenesisHash();
  if (genesis !== MAINNET_GENESIS) {
    throw new Error(`RPC genesis ${genesis} is not mainnet fork`);
  }

  const symbol = opts.symbol ?? "SOL";
  const rise = await import("@ellipsis-labs/rise");
  const client = rise.createPhoenixClient({
    apiUrl: API,
    rpcUrl,
    ws: false,
    exchangeMetadata: { stream: false },
  });
  await client.exchange.ready();

  const market = await client.api.markets().getMarket(symbol);
  if (market.isolatedOnly) {
    throw new Error(`skip isolatedOnly market ${symbol}`);
  }

  const trader = await rise.fetchTrader({
    client: client.rpc.accounts,
    address: opts.traderPda.toBase58() as never,
    skipCache: true,
  });

  let residualLots = 0n;
  let poolUnsettledQuote = 0n;
  const currentAcc = market.statsSnapshot
    ? toBigInt(market.statsSnapshot.cumulativeFundingRate)
    : 0n;
  for (const e of trader.positions.entries) {
    const lots = toBigInt(e.value.baseLotPosition);
    residualLots += lots;
    const snap = toBigInt(e.value.cumulativeFundingSnapshot);
    poolUnsettledQuote += (currentAcc - snap) * lots;
  }

  const poolUnsettledUsdc = quoteLotsToUsdc(poolUnsettledQuote, USDC_DECIMALS);
  const phoenixCollateral = quoteLotsToUsdc(
    toBigInt(trader.state.quoteLotCollateral),
    USDC_DECIMALS
  );
  const usdcPerLot =
    residualLots === 0n ? 0n : poolUnsettledUsdc / residualLots;
  const poolDelta = usdcPerLot * residualLots;
  const dustAbs =
    poolUnsettledUsdc >= poolDelta
      ? poolUnsettledUsdc - poolDelta
      : poolDelta - poolUnsettledUsdc;

  client.dispose();

  return {
    interval: {
      assetId: market.assetId,
      usdcPerLot: usdcPerLot.toString(),
      poolUnsettledUsdc: poolUnsettledUsdc.toString(),
      phoenixCollateral: phoenixCollateral.toString(),
    },
    isolatedOnly: market.isolatedOnly,
    fundingIntervalSeconds: market.fundingIntervalSeconds,
    fundingPeriodSeconds: market.fundingPeriodSeconds,
    residualLots: residualLots.toString(),
    dustAbs: dustAbs.toString(),
  };
}

export { DUST_CAP };
