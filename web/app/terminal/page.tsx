"use client";

import Image from "next/image";
import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { PhoenixClient } from "@ellipsis-labs/rise";

type ScenarioId = "offset" | "residual" | "reject";
type Candle = { time: number; open: number; high: number; low: number; close: number; volume: number };
type Level = [number, number];
type MarketData = {
  candles: Candle[];
  orderbook: { asks: Level[]; bids: Level[]; mid: number | null; symbol: string };
  stats: { annualizedFundingRate: number; dayVolumeUsd: number; markPrice: number; openInterest: number; previousDayMarkPrice: number };
  updatedAt: number;
};

const scenarios: Record<ScenarioId, { label: string; status: string; privateOrders: Array<[string, string, string]>; residual: string; phoenix: string; result: string; tone: "success" | "neutral" | "warning" }> = {
  offset: { label: "Offsetting users", status: "Two private fills reconcile to zero venue exposure.", privateOrders: [["Alice", "BUY", "+10 SOL"], ["Bob", "SELL", "−10 SOL"]], residual: "0 SOL", phoenix: "0 SOL", result: "Both fills acknowledged · pooled position is flat", tone: "success" },
  residual: { label: "Residual exposure", status: "Cinder routes the remaining pooled risk through its shared trader.", privateOrders: [["Alice", "BUY", "+10 SOL"], ["Bob", "SELL", "−4 SOL"]], residual: "+6 SOL", phoenix: "+6 SOL", result: "Both fills acknowledged · Phoenix sees the +6 SOL net only", tone: "neutral" },
  reject: { label: "Venue rejection", status: "Tentative state is released when Phoenix does not fill the residual.", privateOrders: [["Alice", "BUY", "+10 SOL"], ["Phoenix", "REJECT", "0 SOL"]], residual: "0 SOL", phoenix: "0 SOL", result: "No fill acknowledged · Alice’s tentative order is released", tone: "warning" },
};

const money = new Intl.NumberFormat("en-US", { style: "currency", currency: "USD", maximumFractionDigits: 2 });
const compact = new Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 2 });
const number = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 });

function Brand() {
  return <Link className="terminal-brand" href="/" aria-label="Back to Cinder home"><Image src="/images/logo.png" alt="" width={32} height={32} priority /><Image src="/images/name.png" alt="Cinder" width={104} height={34} priority /></Link>;
}

function CandlestickChart({ candles, lastPrice }: { candles: Candle[]; lastPrice: number }) {
  const visible = candles.slice(-72);
  const width = 1000;
  const height = 420;
  const top = 24;
  const bottom = 32;
  const values = visible.flatMap((candle) => [candle.high, candle.low]);
  const min = values.length ? Math.min(...values) : lastPrice * .96;
  const max = values.length ? Math.max(...values) : lastPrice * 1.04;
  const padding = Math.max((max - min) * .08, .01);
  const floor = min - padding;
  const ceiling = max + padding;
  const plotHeight = height - top - bottom;
  const y = (price: number) => top + (ceiling - price) / (ceiling - floor) * plotHeight;
  const step = (width - 28) / Math.max(visible.length, 1);
  const bodyWidth = Math.max(2, step * .62);
  const labels = Array.from({ length: 5 }, (_, index) => ceiling - index * (ceiling - floor) / 4);

  if (!candles.length) return <div className="chart-skeleton" aria-label="Loading Phoenix chart" />;

  return <div className="live-chart" role="img" aria-label="Live one-hour SOL perpetual candlestick chart from Phoenix">
    <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" aria-hidden="true">
      {labels.map((label) => <line className="chart-grid" key={label} x1="0" x2={width} y1={y(label)} y2={y(label)} />)}
      {visible.map((candle, index) => {
        const x = 8 + index * step + step / 2;
        const up = candle.close >= candle.open;
        const rectTop = y(Math.max(candle.open, candle.close));
        const rectHeight = Math.max(1.5, Math.abs(y(candle.open) - y(candle.close)));
        return <g key={candle.time} className={up ? "candle up" : "candle down"}><line x1={x} x2={x} y1={y(candle.high)} y2={y(candle.low)} /><rect x={x - bodyWidth / 2} y={rectTop} width={bodyWidth} height={rectHeight} /></g>;
      })}
      <line className="last-price-line" x1="0" x2={width} y1={y(lastPrice)} y2={y(lastPrice)} />
    </svg>
    <span className="chart-live-price" style={{ top: `${Math.min(91, Math.max(5, (y(lastPrice) / height) * 100))}%` }}>{money.format(lastPrice)}</span>
    <div className="chart-y-axis" aria-hidden="true">{labels.map((label) => <span key={label}>{money.format(label)}</span>)}</div>
  </div>;
}

function BookRows({ side, rows }: { side: "ask" | "bid"; rows: Level[] }) {
  return <div className={`book-rows ${side}`}>{rows.slice(0, 9).map(([price, size], index) => <div className="book-row" key={`${price}-${index}`}><span>{number.format(price)}</span><span>{number.format(size)}</span></div>)}</div>;
}

export default function TerminalPage() {
  const [activeScenario, setActiveScenario] = useState<ScenarioId>("offset");
  const [market, setMarket] = useState<MarketData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [wsLive, setWsLive] = useState(false);
  const scenario = scenarios[activeScenario];

  const loadMarket = useCallback(async () => {
    try {
      setError(null);
      const response = await fetch("/api/market", { cache: "no-store" });
      const data = await response.json() as MarketData & { error?: string };
      if (!response.ok) throw new Error(data.error ?? "Phoenix market data is unavailable.");
      setMarket(data);
    } catch (requestError) {
      setError(requestError instanceof Error ? requestError.message : "Phoenix market data is unavailable.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadMarket();
    const polling = window.setInterval(() => void loadMarket(), 2_000);
    return () => window.clearInterval(polling);
  }, [loadMarket]);

  useEffect(() => {
    const controller = new AbortController();
    let disposed = false;
    void (async () => {
      let client: PhoenixClient | undefined;
      try {
        const { createPhoenixClient } = await import("@ellipsis-labs/rise");
        client = createPhoenixClient({ apiUrl: "https://perp-api.phoenix.trade", ws: { connectMode: "eager" }, exchangeMetadata: { stream: false } });
        const stream = client.streams?.l2Book("SOL-PERP", controller.signal);
        if (!stream) return;
        for await (const update of stream) {
          if (disposed) return;
          setMarket((current) => current ? { ...current, orderbook: { ...current.orderbook, asks: update.asks, bids: update.bids, mid: (update.asks[0]?.[0] + update.bids[0]?.[0]) / 2 || current.orderbook.mid }, updatedAt: Date.now() } : current);
          setWsLive(true);
        }
      } catch {
        if (!disposed) setWsLive(false);
      } finally {
        client?.dispose();
      }
    })();
    return () => { disposed = true; controller.abort(); };
  }, []);

  const mark = market?.stats.markPrice ?? market?.orderbook.mid ?? 0;
  const change = market ? (market.stats.markPrice / market.stats.previousDayMarkPrice - 1) * 100 : 0;
  const spread = market?.orderbook.asks[0] && market.orderbook.bids[0] ? market.orderbook.asks[0][0] - market.orderbook.bids[0][0] : 0;
  const status = wsLive ? "LIVE" : loading ? "LOADING" : "POLLING";
  const statusTone = wsLive ? "live" : error ? "error" : "waiting";
  const marketRows = useMemo(() => market ? [["Mark", money.format(market.stats.markPrice)], ["24h change", `${change >= 0 ? "+" : ""}${change.toFixed(2)}%`], ["24h volume", money.format(market.stats.dayVolumeUsd)], ["Open interest", compact.format(market.stats.openInterest)], ["Funding", `${(market.stats.annualizedFundingRate * 100).toFixed(4)}%`]] : [], [market, change]);

  return <main className="phoenix-terminal">
    <a className="skip-link" href="#terminal-content">Skip to terminal content</a>
    <header className="trade-header"><Brand /><nav aria-label="Terminal navigation"><Link className="active" href="/terminal">Trade</Link><Link href="/#how-it-works">How it works</Link><Link href="/#privacy">Privacy</Link></nav><div className="trade-header-actions"><span className={`feed-status ${statusTone}`}><i aria-hidden="true" /> {status}</span><Link href="/" className="about-link">About Cinder</Link></div></header>
    <section className="market-ticker" aria-label="Phoenix SOL perpetual market statistics"><div className="ticker-symbol"><span className="sol-mark" aria-hidden="true">S</span><strong>SOL</strong><span>PERP</span></div>{loading ? <div className="ticker-loading" aria-label="Loading Phoenix market data" /> : marketRows.map(([label, value]) => <div className="ticker-stat" key={label}><span>{label}</span><strong className={label === "24h change" && change < 0 ? "negative" : label === "24h change" ? "positive" : ""}>{value}</strong></div>)}</section>
    <section className="prototype-banner"><strong>READ-ONLY PHOENIX MARKET DATA</strong><span>Live price and order-book context; Cinder execution panels remain a local prototype with no wallet or order routing.</span>{error && <button type="button" onClick={() => { setLoading(true); void loadMarket(); }}>Retry feed</button>}</section>
    <div className="trade-layout" id="terminal-content">
    <section className="chart-area" aria-labelledby="chart-title"><div className="trade-panel-heading"><div><p>PHOENIX MARKET</p><h1 id="chart-title">SOL-PERP · 1H</h1></div><div className="chart-tools"><span>1H</span><span>Mark price</span><span>{market ? `Updated ${new Date(market.updatedAt).toLocaleTimeString()}` : "Connecting"}</span></div></div>{error && !market ? <div className="feed-error"><strong>Couldn’t load Phoenix market data</strong><p>{error}</p><button type="button" onClick={() => { setLoading(true); void loadMarket(); }}>Try again</button></div> : <><CandlestickChart candles={market?.candles ?? []} lastPrice={mark} /><div className="chart-footer"><span><i aria-hidden="true" /> {wsLive ? "Live order-book stream connected" : "Public snapshot refreshes every 2 seconds"}</span><span>Source: Phoenix</span></div></>}</section>
      <aside className="order-entry" aria-labelledby="order-title"><div className="order-top"><span>Cinder account</span><button type="button" className="account-kind">Cross · demo</button></div><div className="order-tabs"><button type="button" className="selected">Market intent</button><button type="button" disabled>Limit</button></div><div className="intent-toggle"><button type="button" className="long-intent">Long / Buy</button><button type="button" className="short-intent">Short / Sell</button></div><div className="order-balance"><span>Private demo collateral</span><strong>$25,000.00</strong></div><div className="demo-order"><p>CHOOSE A CINDER WALKTHROUGH</p><h2 id="order-title">Private execution intent</h2>{(Object.keys(scenarios) as ScenarioId[]).map((id) => <button key={id} className={activeScenario === id ? "scenario-choice active" : "scenario-choice"} type="button" onClick={() => setActiveScenario(id)} aria-pressed={activeScenario === id}>{scenarios[id].label}<span>›</span></button>)}</div><div className="order-disabled"><span aria-hidden="true">⊘</span><p>Order routing is disabled. This terminal never requests a wallet signature.</p></div></aside>
      <section className="orderbook-area" aria-labelledby="book-title"><div className="subpanel-heading"><h2 id="book-title">Phoenix order book</h2><span>PRICE · SIZE</span></div>{loading ? <div className="book-skeleton" aria-label="Loading Phoenix order book" /> : <><BookRows side="ask" rows={market?.orderbook.asks ?? []} /><div className="book-mid"><strong>{money.format(mark)}</strong><span>Spread {number.format(spread)}</span></div><BookRows side="bid" rows={market?.orderbook.bids ?? []} /></>}<p className="book-source">{wsLive ? "Live WebSocket updates" : "HTTP snapshot"}</p></section>
      <section className="cinder-activity" aria-labelledby="scenario-title"><div className="activity-tabs"><span className="active">Cinder reconciliation</span><span>Private intents</span><span>Pooled residual</span><span>Phoenix acknowledgement</span></div><div className="scenario-content"><div><p className="panel-kicker">DETERMINISTIC CINDER PROTOTYPE</p><h2 id="scenario-title">{scenario.label}</h2><p className="scenario-description" aria-live="polite">{scenario.status}</p></div><span className={`scenario-status ${scenario.tone}`}>{scenario.tone === "warning" ? "RELEASED" : "ACKNOWLEDGED"}</span></div><div className="reconciliation-flow"><div className="flow-stage"><span className="flow-label">PRIVATE INTENTS</span>{scenario.privateOrders.map(([actor, action, size]) => <div className="intent-row" key={`${actor}-${action}`}><span>{actor}</span><strong className={action === "BUY" ? "buy" : action === "SELL" ? "sell" : "reject"}>{action}</strong><span>{size}</span></div>)}</div><div className="flow-arrow" aria-hidden="true">→</div><div className="flow-stage"><span className="flow-label">CINDER NET</span><strong className="flow-number">{scenario.residual}</strong><small>aggregate Book after acknowledgement</small></div><div className="flow-arrow" aria-hidden="true">→</div><div className="flow-stage phoenix-flow"><span className="flow-label">PHOENIX SEES</span><strong className="flow-number">{scenario.phoenix}</strong><small>one shared cross trader</small></div></div><div className={`scenario-result ${scenario.tone}`}><span aria-hidden="true">✓</span>{scenario.result}</div></section>
    </div>
  </main>;
}
