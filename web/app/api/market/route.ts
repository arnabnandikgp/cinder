import { PhoenixHttpClient } from "@ellipsis-labs/rise";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

const apiUrl = process.env.PHOENIX_API_URL ?? "https://perp-api.phoenix.trade";
const cacheHeaders = {
  "Cache-Control": "public, max-age=0, must-revalidate",
  "CDN-Cache-Control": "s-maxage=2, stale-while-revalidate=8",
  "Vercel-CDN-Cache-Control": "s-maxage=2, stale-while-revalidate=8",
};

function normalizeLevels(
  levels: readonly [number, number][],
  side: "ask" | "bid",
): [number, number][] {
  const byPrice = new Map<number, number>();

  for (const [price, size] of levels) {
    if (!Number.isFinite(price) || !Number.isFinite(size) || price <= 0 || size <= 0) continue;
    byPrice.set(price, size);
  }

  return [...byPrice.entries()].sort(([left], [right]) =>
    side === "ask" ? left - right : right - left,
  );
}

export async function GET() {
  const client = new PhoenixHttpClient({ apiUrl, timeout: 8_000 });

  try {
    const [candles, orderbook, stats] = await Promise.all([
      client.candles().getCandles("SOL", { timeframe: "1h", limit: 96 }),
      client.orderbook().getOrderbook("SOL"),
      client.markets().getLatestMarketStats("SOL"),
    ]);

    const asks = normalizeLevels(orderbook.asks, "ask");
    const bids = normalizeLevels(orderbook.bids, "bid");

    return Response.json(
      {
        candles,
        orderbook: {
          asks,
          bids,
          mid: orderbook.mid ?? ((asks[0]?.[0] + bids[0]?.[0]) / 2 || null),
          symbol: orderbook.symbol,
        },
        stats: {
          annualizedFundingRate: stats.annualized_funding_rate,
          dayVolumeUsd: stats.day_volume_usd,
          markPrice: stats.mark_price,
          openInterest: stats.open_interest,
          previousDayMarkPrice: stats.prev_day_mark_price,
        },
        updatedAt: Date.now(),
      },
      { headers: cacheHeaders },
    );
  } catch {
    return Response.json(
      { error: "Phoenix market data is temporarily unavailable. Try again in a moment." },
      { status: 502, headers: { "Cache-Control": "no-store" } },
    );
  } finally {
    client.dispose();
  }
}
