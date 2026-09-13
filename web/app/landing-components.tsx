"use client";

import { useState } from "react";
import Link from "next/link";

/* -------------------------------------------------------------------------
   1. HERO VISUALIZER
------------------------------------------------------------------------- */
export function HeroVisualizer() {
  const [activePreset, setActivePreset] = useState<"offset" | "residual">("offset");

  const isOffset = activePreset === "offset";

  return (
    <div className="hero-sandbox" aria-label="Interactive Cinder Architecture Preview">
      <div className="sandbox-header">
        <div className="sandbox-status-dot">
          <span className="live-pulse" />
          <span>MAGICBLOCK PER · REAL-TIME RECONCILIATION</span>
        </div>
        <div className="sandbox-toggle">
          <button
            type="button"
            className={isOffset ? "active" : ""}
            onClick={() => setActivePreset("offset")}
            aria-pressed={isOffset}
          >
            Net Zero Offset
          </button>
          <button
            type="button"
            className={!isOffset ? "active" : ""}
            onClick={() => setActivePreset("residual")}
            aria-pressed={!isOffset}
          >
            Residual Flow
          </button>
        </div>
      </div>

      <div className="sandbox-grid">
        {/* Column 1: Confidential User Ledgers */}
        <div className="sandbox-col">
          <div className="col-badge">
            <span className="badge-shield">🛡</span>
            <span>MAGICBLOCK PER (PRIVATE)</span>
          </div>

          <div className="trader-cards">
            <div className="trader-card buyer">
              <div className="trader-info">
                <span className="trader-name">Trader Alice</span>
                <span className="trader-badge buy">BUY INTENT</span>
              </div>
              <div className="trader-val">
                <strong>+10.0 SOL</strong>
                <span>Margin: $1,500 USDC</span>
              </div>
              <div className="privacy-pill">
                <span>QFS Token Authenticated</span>
              </div>
            </div>

            <div className="trader-card seller">
              <div className="trader-info">
                <span className="trader-name">Trader Bob</span>
                <span className="trader-badge sell">SELL INTENT</span>
              </div>
              <div className="trader-val">
                <strong>{isOffset ? "−10.0 SOL" : "−4.0 SOL"}</strong>
                <span>Margin: {isOffset ? "$1,500 USDC" : "$600 USDC"}</span>
              </div>
              <div className="privacy-pill">
                <span>QFS Token Authenticated</span>
              </div>
            </div>
          </div>
        </div>

        {/* Center: Cinder Netting Adapter */}
        <div className="sandbox-hub">
          <div className="hub-core">
            <div className="hub-icon">✦</div>
            <div className="hub-title">CINDER NETTING</div>
            <div className="hub-equation">
              {isOffset ? "+10 − 10 = 0" : "+10 − 4 = +6"}
            </div>
            <div className="hub-tag">
              {isOffset ? "100% INTERNAL OFFSET" : "RESIDUAL ROUTING"}
            </div>
            <div className="hub-metric">Immediate Netting · Window = 0</div>
          </div>
        </div>

        {/* Column 3: Phoenix CLOB */}
        <div className="sandbox-col venue-col">
          <div className="col-badge venue-badge">
            <span className="badge-pulse">⚡</span>
            <span>PHOENIX CLOB (SOLANA L1)</span>
          </div>

          <div className="venue-card">
            <div className="venue-header">
              <span className="venue-title">Shared Cross Trader</span>
              <span className="anonymous-badge">1 VENUE ID</span>
            </div>
            <div className="venue-display">
              <span className="venue-label">Public Net Exposure</span>
              <strong className={isOffset ? "net-zero" : "net-pos"}>
                {isOffset ? "0.00 SOL" : "+6.00 SOL"}
              </strong>
              <p className="venue-desc">
                {isOffset
                  ? "Zero orders routed to Phoenix. Internalized with zero venue slippage and zero counterparty footprint."
                  : "Only the residual +6 SOL routed. Individual user identities and position sizes remain 100% private."}
              </p>
            </div>
            <div className="venue-footer">
              <span>Attributable Users: 0</span>
              <span>Matching Engine: Phoenix SOL/USDC</span>
            </div>
          </div>
        </div>
      </div>

      <div className="sandbox-bar">
        <div className="bar-item">
          <span className="bar-dot green" />
          <span>User Positions: Confidential in Ephemeral Rollup</span>
        </div>
        <div className="bar-item">
          <span className="bar-dot amber" />
          <span>Two-Phase Commit: No phantom risk</span>
        </div>
        <div className="bar-item">
          <span className="bar-dot cyan" />
          <span>Single Phoenix Trader: Pooled liquidity</span>
        </div>
      </div>
    </div>
  );
}

/* -------------------------------------------------------------------------
   2. INTERACTIVE NETTING SIMULATOR
------------------------------------------------------------------------- */
type ScenarioKey = "offset" | "residual" | "rejection";

interface ScenarioData {
  title: string;
  tagline: string;
  traders: Array<{ name: string; side: "BUY" | "SELL"; size: string; margin: string }>;
  netResidual: string;
  phoenixOrder: string;
  status: string;
  statusType: "success" | "neutral" | "warning";
  explanation: string;
  actionCode: string;
}

const SCENARIOS: Record<ScenarioKey, ScenarioData> = {
  offset: {
    title: "1. Perfect Internal Netting",
    tagline: "Offsetting user trades reconcile privately with zero venue exposure.",
    traders: [
      { name: "Alice (Ledger A)", side: "BUY", size: "+10.0 SOL", margin: "$1,800 USDC" },
      { name: "Bob (Ledger B)", side: "SELL", size: "−10.0 SOL", margin: "$1,800 USDC" },
    ],
    netResidual: "0.00 SOL",
    phoenixOrder: "0 SOL (No Order Placed)",
    status: "ACKNOWLEDGED · 100% INTERNALIZED",
    statusType: "success",
    explanation:
      "Alice and Bob's counter-intents match exactly inside Cinder's Book. No transaction is submitted to Phoenix, preventing frontrunning, eliminating taker fees, and leaking zero information to Solana explorers.",
    actionCode: "ack_phoenix_fill(batch_id, residual = 0)",
  },
  residual: {
    title: "2. Pooled Residual Execution",
    tagline: "Unbalanced private orders aggregate into a single anonymous venue trade.",
    traders: [
      { name: "Alice (Ledger A)", side: "BUY", size: "+15.0 SOL", margin: "$2,700 USDC" },
      { name: "Bob (Ledger B)", side: "SELL", size: "−5.0 SOL", margin: "$900 USDC" },
    ],
    netResidual: "+10.0 SOL",
    phoenixOrder: "BUY +10.0 SOL (Limit/Market)",
    status: "ACKNOWLEDGED · RESIDUAL CONFIRMED",
    statusType: "neutral",
    explanation:
      "Cinder offsets the 5 SOL internally and sends only the remaining +10 SOL residual to Phoenix via Cinder's shared cross trader. Phoenix sees one single order for 10 SOL and has no visibility into Alice or Bob.",
    actionCode: "ack_phoenix_fill(batch_id, residual = +10)",
  },
  rejection: {
    title: "3. Venue Rejection & Rollback Safety",
    tagline: "Tentative positions safely release if Phoenix liquidity fails or rejects.",
    traders: [
      { name: "Alice (Ledger A)", side: "BUY", size: "+10.0 SOL", margin: "$1,800 USDC (Reserved)" },
    ],
    netResidual: "+10.0 SOL (Attempted)",
    phoenixOrder: "REJECTED (Insufficient Liquidity / IOC)",
    status: "RELEASED · TENTATIVE STATE ROLLED BACK",
    statusType: "warning",
    explanation:
      "A fill is acknowledged, not assumed. Because Phoenix rejected the venue order, Cinder executes ack_phoenix_fail, immediately releasing Alice's reserved margin. The aggregate book never claims phantom exposure.",
    actionCode: "ack_phoenix_fail(order_id, reason = IOC_UNFILLED)",
  },
};

export function NettingSimulator() {
  const [selectedKey, setSelectedKey] = useState<ScenarioKey>("offset");
  const active = SCENARIOS[selectedKey];

  return (
    <div className="simulator-box" id="netting-simulator">
      <div className="simulator-nav">
        {(["offset", "residual", "rejection"] as ScenarioKey[]).map((key) => {
          const item = SCENARIOS[key];
          return (
            <button
              key={key}
              type="button"
              className={`sim-nav-btn ${selectedKey === key ? "active" : ""}`}
              onClick={() => setSelectedKey(key)}
              aria-pressed={selectedKey === key}
            >
              <span className="sim-step">{key === "offset" ? "01" : key === "residual" ? "02" : "03"}</span>
              <span className="sim-title">{item.title.split(". ")[1]}</span>
            </button>
          );
        })}
      </div>

      <div className="simulator-body">
        <div className="sim-top-banner">
          <div>
            <span className="sim-eyebrow">ACTIVE SIMULATION</span>
            <h3 className="sim-heading">{active.title}</h3>
            <p className="sim-tagline">{active.tagline}</p>
          </div>
          <span className={`sim-status-badge ${active.statusType}`}>
            {active.status}
          </span>
        </div>

        <div className="sim-visual-grid">
          {/* Box 1: Confidential Intent Inputs */}
          <div className="sim-panel">
            <div className="sim-panel-header">
              <span>1. PRIVATE INTENT (MAGICBLOCK PER)</span>
              <span className="sub-badge">Encrypted Ledger</span>
            </div>
            <div className="sim-traders-list">
              {active.traders.map((trader) => (
                <div key={trader.name} className="sim-trader-row">
                  <div className="trader-meta">
                    <strong>{trader.name}</strong>
                    <span>{trader.margin}</span>
                  </div>
                  <div className={`trader-badge ${trader.side.toLowerCase()}`}>
                    {trader.side} {trader.size}
                  </div>
                </div>
              ))}
            </div>
            <div className="sim-panel-footer">
              <span>Tentative size recorded · Margin locked in ER</span>
            </div>
          </div>

          {/* Arrow */}
          <div className="sim-arrow" aria-hidden="true">
            <span>⟶</span>
            <small>Netting</small>
          </div>

          {/* Box 2: Cinder Reconciliation */}
          <div className="sim-panel highlight">
            <div className="sim-panel-header">
              <span>2. CINDER ADAPTER RECONCILIATION</span>
              <span className="sub-badge glow">Window = 0</span>
            </div>
            <div className="sim-reconcile-content">
              <span className="reconcile-label">Calculated Residual Risk</span>
              <strong className="reconcile-val">{active.netResidual}</strong>
              <div className="sim-code-chip">
                <code>{active.actionCode}</code>
              </div>
            </div>
            <div className="sim-panel-footer">
              <span>Single Phoenix Cross Account (PDA)</span>
            </div>
          </div>

          {/* Arrow */}
          <div className="sim-arrow" aria-hidden="true">
            <span>⟶</span>
            <small>Routing</small>
          </div>

          {/* Box 3: Public Venue Output */}
          <div className="sim-panel venue">
            <div className="sim-panel-header">
              <span>3. PHOENIX CLOB (SOLANA L1)</span>
              <span className="sub-badge">Public Orderbook</span>
            </div>
            <div className="sim-reconcile-content">
              <span className="reconcile-label">Venue Observed Exposure</span>
              <strong className="phoenix-val">{active.phoenixOrder}</strong>
              <div className="sim-stats-mini">
                <div><span>Footprint:</span><strong>Single Trader</strong></div>
                <div><span>User Leakage:</span><strong>0 bytes</strong></div>
              </div>
            </div>
            <div className="sim-panel-footer">
              <span>Confirmed fill updates aggregate Book</span>
            </div>
          </div>
        </div>

        <div className="sim-bottom-insight">
          <div className="insight-icon">💡</div>
          <div className="insight-text">
            <strong>Protocol Architecture Insight:</strong> {active.explanation}
          </div>
          <Link href="/terminal" className="insight-btn">
            Test in Live Terminal ↗
          </Link>
        </div>
      </div>
    </div>
  );
}

/* -------------------------------------------------------------------------
   3. ARCHITECTURE 3-TIER TABS
------------------------------------------------------------------------- */
type LayerKey = "per" | "adapter" | "solana";

export function ArchitectureTabs() {
  const [activeLayer, setActiveLayer] = useState<LayerKey>("per");

  const layers: Record<
    LayerKey,
    {
      name: string;
      subtitle: string;
      badge: string;
      points: Array<{ title: string; desc: string; icon: string }>;
      codeSnippet: string;
    }
  > = {
    per: {
      name: "MagicBlock PER",
      subtitle: "Private Ephemeral Rollup for confidential state",
      badge: "CONFIDENTIAL LAYER",
      points: [
        {
          icon: "🔒",
          title: "UserLedger PDAs",
          desc: "Each trader's position, reserved margin balance, entry price, and unrealized PnL are isolated inside private rollup accounts.",
        },
        {
          icon: "⚡",
          title: "Microsecond Execution",
          desc: "Orders and margin adjustments process immediately without contending with Solana L1 block gas or public mempool congestion.",
        },
        {
          icon: "🛡",
          title: "QFS Token Verification",
          desc: "Query Filtering Service authenticates traders with short-lived tokens, preventing unauthorized observers from scraping positions.",
        },
      ],
      codeSnippet: `// Private Ephemeral Rollup Account
#[account]
pub struct UserLedger {
    pub owner: Pubkey,
    pub margin_balance: u64,
    pub reserved_margin: u64,
    pub net_position: i64,      // strictly private
    pub tentative_position: i64, // pending venue ack
    pub entry_price_scaled: u64,
}`,
    },
    adapter: {
      name: "Operator Adapter",
      subtitle: "Reconciles the aggregate book and executes venue fills",
      badge: "RECONCILIATION ENGINE",
      points: [
        {
          icon: "⚖️",
          title: "Immediate Residual Netting",
          desc: "Aggregates internal long and short user intents instantaneously (window = 0) to compute the minimal residual required at Phoenix.",
        },
        {
          icon: "🔄",
          title: "Two-Phase Fill Confirmation",
          desc: "Positions are never treated as filled until Phoenix issues confirmation. Fill confirmation invokes ack_phoenix_fill.",
        },
        {
          icon: "🚫",
          title: "Zero Token Persistence",
          desc: "The adapter uses a single operator credential to access reconcilable accounts and never stores or persists user QFS tokens.",
        },
      ],
      codeSnippet: `// Adapter Execution Flow
pub async fn reconcile_batch(&self) -> Result<()> {
    let residual = self.compute_net_residual()?;
    if residual != 0 {
        let phoenix_sig = self.route_phoenix_order(residual).await?;
        self.ack_phoenix_fill(batch_id, phoenix_sig).await?;
    } else {
        self.ack_internal_offset(batch_id).await?;
    }
    Ok(())
}`,
    },
    solana: {
      name: "Solana L1 & Phoenix",
      subtitle: "Public settlement, collateral custody, and CLOB liquidity",
      badge: "PUBLIC CONSENSUS",
      points: [
        {
          icon: "🏦",
          title: "Non-Custodial Vault Program",
          desc: "Manages on-chain collateral deposits and withdrawals. Users deposit USDC directly to protocol vault PDAs on Solana L1.",
        },
        {
          icon: "🦅",
          title: "One Phoenix Cross Trader",
          desc: "All external orders originate from Cinder's single cross-margin account. Phoenix matches the aggregate volume anonymously.",
        },
        {
          icon: "📐",
          title: "Settlement & Invariant Auditing",
          desc: "Cryptographic invariants ensure that the sum of all private PER ledgers matches Cinder's Phoenix balance to the exact lamport.",
        },
      ],
      codeSnippet: `// Phoenix Single Cross Trader Order
let order_packet = MarketOrderPacket {
    side: if residual > 0 { Side::Bid } else { Side::Ask },
    num_base_lots: residual.unsigned_abs(),
    client_order_id: cinder_nonce,
    self_trade_behavior: SelfTradeBehavior::Abort,
};
phoenix::send_order(&cinder_cross_trader, order_packet)?;`,
    },
  };

  const current = layers[activeLayer];

  return (
    <div className="arch-tabs-wrapper">
      <div className="arch-layer-buttons">
        {(["per", "adapter", "solana"] as LayerKey[]).map((key) => {
          const l = layers[key];
          return (
            <button
              key={key}
              type="button"
              className={`layer-btn ${activeLayer === key ? "active" : ""}`}
              onClick={() => setActiveLayer(key)}
              aria-pressed={activeLayer === key}
            >
              <div className="layer-btn-top">
                <span className="layer-badge">{l.badge}</span>
              </div>
              <strong className="layer-btn-title">{l.name}</strong>
              <span className="layer-btn-desc">{l.subtitle}</span>
            </button>
          );
        })}
      </div>

      <div className="arch-tab-content">
        <div className="arch-points-col">
          <div className="points-header">
            <h3>{current.name}</h3>
            <p>{current.subtitle}</p>
          </div>
          <div className="points-list">
            {current.points.map((pt) => (
              <div key={pt.title} className="arch-point-item">
                <span className="point-icon">{pt.icon}</span>
                <div>
                  <strong>{pt.title}</strong>
                  <p>{pt.desc}</p>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="arch-code-col">
          <div className="code-header">
            <span className="code-dot red" />
            <span className="code-dot yellow" />
            <span className="code-dot green" />
            <span className="code-title">cinder_core · {activeLayer}.rs</span>
          </div>
          <pre className="arch-code-block">
            <code>{current.codeSnippet}</code>
          </pre>
        </div>
      </div>
    </div>
  );
}

/* -------------------------------------------------------------------------
   4. TECHNICAL FAQ ACCORDION
------------------------------------------------------------------------- */
interface FaqItem {
  q: string;
  a: string;
  category: string;
}

const FAQ_ITEMS: FaqItem[] = [
  {
    category: "Privacy & Security",
    q: "How does Cinder prevent other traders or bots from reading my positions?",
    a: "User state is stored inside MagicBlock Private Ephemeral Rollups. Access is authenticated through the Query Filtering Service (QFS) using short-lived cryptographic tokens. A trader can authenticate only to inspect their own UserLedger account. No public RPC node or block explorer can query individual trader positions.",
  },
  {
    category: "Architecture",
    q: "Why does Cinder use a single Phoenix cross trader account?",
    a: "Creating per-user accounts at Phoenix would completely destroy privacy by exposing individual wallet addresses, execution timestamps, and fill sizes on Solana L1. By pooling all users into one Cinder cross trader, Phoenix sees only the collective net exposure, blending all trader activity into an indivisible aggregate footprint.",
  },
  {
    category: "Execution Integrity",
    q: "What happens if a residual order is rejected or fails to fill on Phoenix?",
    a: "Cinder adheres strictly to a two-phase confirmation protocol: order intent is tentative until verified by Phoenix. If an order cannot be filled (e.g. price limit crossed or IOC cancel), the adapter executes ack_phoenix_fail. This immediately rolls back the tentative order and unlocks reserved user margin. The book never assumes phantom risk.",
  },
  {
    category: "Netting & Efficiency",
    q: "What does 'Window = 0' immediate residual netting mean?",
    a: "Unlike delayed batch auctions that force users to wait seconds or minutes for netting rounds, Cinder computes residual obligations immediately upon order submission. If counter-orders exist within the pool, they cross internally with zero delay and zero venue taker fees.",
  },
  {
    category: "Safety & Invariants",
    q: "How does the protocol guarantee solvency across the private rollup and Solana L1?",
    a: "Cinder continuously checks two core mathematical invariants: I1 (Position Integrity: Sum of all private positions must equal Phoenix's position) and I2 (Collateral Integrity: User cash + unrealized PnL equals vault deposits + Phoenix margin equity). If any drift is detected, automated circuit breakers immediately HALT new risk.",
  },
  {
    category: "Development & Testing",
    q: "Can I run this codebase and test the privacy and netting mechanics locally?",
    a: "Yes! The repository is open source and comes with complete local test suites. You can run ./scripts/test-ledger.sh to test deterministic state transitions, ./scripts/stack-local.sh to launch the QFS privacy endpoint, and explore the interactive /terminal demo.",
  },
];

export function FaqAccordion() {
  const [openIndex, setOpenIndex] = useState<number | null>(0);

  const toggle = (index: number) => {
    setOpenIndex(openIndex === index ? null : index);
  };

  return (
    <div className="faq-wrapper" aria-label="Frequently Asked Technical Questions">
      {FAQ_ITEMS.map((item, index) => {
        const isOpen = openIndex === index;
        return (
          <div key={item.q} className={`faq-card ${isOpen ? "open" : ""}`}>
            <button
              type="button"
              className="faq-question-btn"
              onClick={() => toggle(index)}
              aria-expanded={isOpen}
            >
              <span className="faq-category">{item.category}</span>
              <span className="faq-title">{item.q}</span>
              <span className="faq-icon" aria-hidden="true">
                {isOpen ? "−" : "+"}
              </span>
            </button>
            {isOpen && (
              <div className="faq-answer">
                <p>{item.a}</p>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
