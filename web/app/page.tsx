import Image from "next/image";
import Link from "next/link";
import {
  NettingSimulator,
  ArchitectureTabs,
  FaqAccordion,
} from "./landing-components";

const navigation = [
  ["How it Works", "#how-it-works"],
  ["Netting Engine", "#netting-engine"],
  ["Architecture", "#architecture"],
  ["Privacy Matrix", "#privacy"],
  ["Guarantees", "#invariants"],
  ["FAQ", "#faq"],
];

function Brand() {
  return (
    <Link className="brand" href="#top" aria-label="Cinder home">
      <div className="brand-logo-frame">
        <Image src="/images/logo.png" alt="Cinder logo" width={38} height={38} priority />
      </div>
      <Image
        className="brand-name"
        src="/images/name.png"
        alt="Cinder"
        width={132}
        height={44}
        priority
      />
    </Link>
  );
}

function Eyebrow({ children }: Readonly<{ children: React.ReactNode }>) {
  return <p className="eyebrow">{children}</p>;
}

export default function Home() {
  return (
    <div className="landing-root" id="top">
      <a className="skip-link" href="#content">
        Skip to content
      </a>

      {/* Background ambient lighting effects */}
      <div className="ambient-glow glow-top" aria-hidden="true" />
      <div className="ambient-glow glow-mid" aria-hidden="true" />
      <div className="ambient-grid-overlay" aria-hidden="true" />

      {/* Header */}
      <header className="site-header shell">
        <Brand />
        <nav className="site-nav" aria-label="Main navigation">
          {navigation.map(([label, href]) => (
            <a key={href} href={href}>
              {label}
            </a>
          ))}
        </nav>
        <div className="header-actions">
          <a
            className="github-pill"
            href="https://github.com/arnabnandikgp/cinder"
            target="_blank"
            rel="noreferrer"
            aria-label="Cinder GitHub repository"
          >
            <span className="github-icon" aria-hidden="true">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
              </svg>
            </span>
            <span>GitHub</span>
          </a>
          <Link className="header-cta" href="/terminal">
            Launch Terminal <span aria-hidden="true">↗</span>
          </Link>
        </div>
      </header>

      <main id="content">
        {/* HERO SECTION */}
        <section className="hero shell" aria-labelledby="hero-title">
          <div className="hero-copy">
            <div className="hero-badge">
              <span className="badge-spark">✦</span>
              <span>PRIVATE PERP PRIME BROKER · SOLANA</span>
            </div>
            <h1 id="hero-title">
              Private perpetuals.
              <br />
              <span className="gradient-text">Phoenix liquidity.</span>
            </h1>
            <p className="hero-description">
              Cinder privately accounts for every trader inside MagicBlock Private Ephemeral
              Rollups while Phoenix executes only the pooled residual. Your individual book,
              orders, and leverage stay strictly confidential.
            </p>
            <div className="hero-actions">
              <Link className="button button-primary glow-button" href="/terminal">
                Explore Demo Terminal <span aria-hidden="true">→</span>
              </Link>
              <a className="button button-quiet" href="#netting-engine">
                Interactive Netting <span aria-hidden="true">↓</span>
              </a>
            </div>
            <div className="hero-footnotes">
              <span className="footnote-item">
                <i className="status-dot green" /> Deterministic local prototype
              </span>
              <span className="footnote-item">
                <i className="status-dot amber" /> Zero wallet signature required
              </span>
            </div>
          </div>

          <div className="hero-visual-wrapper">
            <div className="hero-terminal-frame">
              <div className="terminal-window-bar">
                <div className="window-dots">
                  <span className="dot dot-close" />
                  <span className="dot dot-min" />
                  <span className="dot dot-expand" />
                </div>
                <div className="window-badge">
                  <span>MAGICBLOCK PER</span>
                </div>
              </div>
              <div className="hero-terminal-img-wrap">
                <Image
                  src="/images/terminal.png"
                  alt="Cinder Terminal TUI - Private Perpetuals on Solana"
                  width={1630}
                  height={965}
                  priority
                  className="hero-terminal-img"
                />
                <div className="hero-terminal-overlay">
                  <Link href="/terminal" className="terminal-preview-btn">
                    <span>Launch Terminal Demo</span>
                    <span aria-hidden="true">↗</span>
                  </Link>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* PROTOCOL STATS RIBBON */}
        <section className="statement-band" aria-label="Key Protocol Properties">
          <div className="shell statement-grid">
            <div className="stat-card">
              <strong className="stat-val">1 Shared Trader</strong>
              <span className="stat-label">Phoenix sees one pooled cross account</span>
            </div>
            <div className="stat-divider" aria-hidden="true">✦</div>
            <div className="stat-card">
              <strong className="stat-val">0 Venue Leakage</strong>
              <span className="stat-label">User ledgers encrypted in MagicBlock PER</span>
            </div>
            <div className="stat-divider" aria-hidden="true">✦</div>
            <div className="stat-card">
              <strong className="stat-val">Window = 0</strong>
              <span className="stat-label">Immediate netting; zero auction delay</span>
            </div>
            <div className="stat-divider" aria-hidden="true">✦</div>
            <div className="stat-card">
              <strong className="stat-val">Two-Phase Commit</strong>
              <span className="stat-label">Fills confirmed before book updates</span>
            </div>
          </div>
        </section>

        {/* THE EXECUTION GAP (PROBLEM STATEMENT) */}
        <section className="section shell problem-section" id="how-it-works" aria-labelledby="problem-title">
          <Image
            className="corner-accent"
            src="/images/corner_meteor_accent.png"
            alt=""
            width={1448}
            height={1086}
          />
          <div className="section-intro split-intro">
            <div>
              <Eyebrow>THE EXECUTION GAP</Eyebrow>
              <h2 id="problem-title">Public venue execution leaks trader edge.</h2>
            </div>
            <p>
              When accounts trade directly on a public DEX, their wallet attribution, entry timing,
              and liquidations are exposed to toxic counterparty MEV. Cinder changes what reaches
              Phoenix without replacing Phoenix’s high-performance matching engine.
            </p>
          </div>

          <div className="comparison-grid">
            <article className="comparison-card comparison-direct">
              <div className="card-top">
                <p className="card-kicker">DIRECT PUBLIC VENUE</p>
                <span className="badge-risk">Public Footprint</span>
              </div>
              <h3>Every account leaves an unshielded venue footprint.</h3>
              <ul className="comparison-list">
                <li>
                  <span className="list-icon negative">✕</span>
                  <div>
                    <strong>Public Wallet Attribution:</strong> Every trade, size, and timestamp
                    is permanently indexed on Solana block explorers.
                  </div>
                </li>
                <li>
                  <span className="list-icon negative">✕</span>
                  <div>
                    <strong>Toxic Counterparty MEV:</strong> Bots track trader PnL, front-run stop
                    losses, and trigger cascading liquidations.
                  </div>
                </li>
                <li>
                  <span className="list-icon negative">✕</span>
                  <div>
                    <strong>Unnecessary Slippage:</strong> Offsetting users trade against public
                    orderbooks, incurring double taker fees and price impact.
                  </div>
                </li>
              </ul>
            </article>

            <article className="comparison-card comparison-cinder">
              <div className="card-top">
                <p className="card-kicker">THROUGH CINDER BROKERAGE</p>
                <span className="badge-shield-pill">Shielded Residual</span>
              </div>
              <h3>Individual books stay confidential; Phoenix sees one net.</h3>
              <ul className="comparison-list">
                <li>
                  <span className="list-icon positive">✓</span>
                  <div>
                    <strong>Private UserLedgers in PER:</strong> Positions, margin balances, and
                    orders live securely inside MagicBlock Ephemeral Rollups.
                  </div>
                </li>
                <li>
                  <span className="list-icon positive">✓</span>
                  <div>
                    <strong>Single Phoenix Cross Trader:</strong> Phoenix matches only Cinder&apos;s
                    pooled residual; individual users are completely invisible.
                  </div>
                </li>
                <li>
                  <span className="list-icon positive">✓</span>
                  <div>
                    <strong>Zero-Leakage Internalization:</strong> Counter-intents net internally
                    with zero slippage, zero fees, and zero block delay.
                  </div>
                </li>
              </ul>
            </article>
          </div>
        </section>

        {/* INTERACTIVE NETTING ENGINE SIMULATOR */}
        <section className="section netting-section" id="netting-engine" aria-labelledby="netting-title">
          <div className="shell">
            <div className="section-intro centered-intro">
              <Eyebrow>THE CINDER NETTING ENGINE</Eyebrow>
              <h2 id="netting-title">Different private books become one public net.</h2>
              <p>
                Phoenix does not receive a list of Cinder users or their private orders. It
                observes only the residual position through one shared cross trader after
                Cinder&apos;s acknowledgements reconcile.
              </p>
            </div>

            <NettingSimulator />
          </div>
        </section>

        {/* 3-TIER ARCHITECTURE & EXECUTION LIFECYCLE */}
        <section className="section shell lifecycle-section" id="architecture" aria-labelledby="lifecycle-title">
          <Image
            className="cascade-accent"
            src="/images/vertical_cascade.png"
            alt=""
            width={941}
            height={1672}
          />
          <div className="section-intro split-intro">
            <div>
              <Eyebrow>SYSTEM ARCHITECTURE</Eyebrow>
              <h2 id="lifecycle-title">Three execution domains. One unified book.</h2>
            </div>
            <p>
              Cinder cleanly separates private user accounting, aggregate reconciliation,
              and public venue execution across three purpose-built layers.
            </p>
          </div>

          <ArchitectureTabs />

          {/* 4-Step Order Lifecycle Timeline */}
          <div className="lifecycle-timeline-wrapper">
            <div className="timeline-heading">
              <Eyebrow>DETERMINISTIC TWO-PHASE COMMIT</Eyebrow>
              <h3>A venue fill is acknowledged, not assumed.</h3>
              <p>
                Orders are never marked filled until Phoenix confirms the transaction.
                A rejected or cancelled order releases tentative margin without phantom risk.
              </p>
            </div>

            <ol className="lifecycle">
              <li>
                <div className="step-num-pill">01</div>
                <h3>Private Intent Placed</h3>
                <p>
                  A trader submits an order to MagicBlock PER. Size is recorded as tentative
                  and collateral is reserved in the UserLedger.
                </p>
              </li>
              <li>
                <div className="step-num-pill">02</div>
                <h3>Instant Residual Netting</h3>
                <p>
                  The operator adapter computes the aggregate delta across all active intents
                  (window = 0) and determines the venue exposure.
                </p>
              </li>
              <li>
                <div className="step-num-pill">03</div>
                <h3>Phoenix Execution</h3>
                <p>
                  If a non-zero residual exists, the adapter sends an anonymous order through
                  Cinder&apos;s single cross trader account.
                </p>
              </li>
              <li>
                <div className="step-num-pill">04</div>
                <h3>Explicit Acknowledgement</h3>
                <p>
                  A confirmed fill calls <code>ack_phoenix_fill</code>, updating the book.
                  Rejections call <code>ack_phoenix_fail</code>, cleanly releasing tentative state.
                </p>
              </li>
            </ol>
          </div>
        </section>

        {/* PRIVACY BOUNDARY MATRIX */}
        <section className="section privacy-section" id="privacy" aria-labelledby="privacy-title">
          <div className="shell privacy-layout">
            <div className="privacy-copy">
              <Eyebrow>CRYPTOGRAPHIC BOUNDARIES</Eyebrow>
              <h2 id="privacy-title">Private positions. Transparent boundaries.</h2>
              <p>
                Cinder protects position attribution and individual order state. It does not
                falsely claim that every protocol event is invisible: deposits, withdrawals,
                and the pooled Phoenix position remain fully transparent on Solana.
              </p>
              <div className="privacy-callout">
                <span className="callout-icon">🛡</span>
                <div>
                  <strong>QFS Access Control:</strong> Token-gated Query Filtering Service allows
                  each trader to inspect their own ledger while strictly denying access to others.
                </div>
              </div>
            </div>

            <div className="boundary-grid">
              <article className="boundary-card private">
                <div className="boundary-header">
                  <span className="dot-private" />
                  <p className="card-kicker">CONFIDENTIAL (MAGICBLOCK PER)</p>
                </div>
                <h3>Individual positions, margins, and orders</h3>
                <ul className="matrix-list">
                  <li>Individual position size and direction (Long / Short)</li>
                  <li>Reserved margin balances and liquidation prices</li>
                  <li>Individual execution timestamps and order history</li>
                  <li>Internal matching offsets between counterparties</li>
                </ul>
              </article>

              <article className="boundary-card public">
                <div className="boundary-header">
                  <span className="dot-public" />
                  <p className="card-kicker">PUBLIC (SOLANA L1 CONSENSUS)</p>
                </div>
                <h3>Collateral movement and pooled venue risk</h3>
                <ul className="matrix-list">
                  <li>Total protocol collateral stored in Vault program PDAs</li>
                  <li>Cinder&apos;s single Phoenix cross trader position size</li>
                  <li>Aggregate deposits and withdrawal transactions</li>
                  <li>Solana L1 block timestamps and settlement receipts</li>
                </ul>
              </article>
            </div>
          </div>
        </section>

        {/* ENGINEERED FINANCIAL INVARIANTS */}
        <section className="section shell safety-section" id="invariants" aria-labelledby="safety-title">
          <div className="safety-header">
            <Eyebrow>RECONCILIATION FIRST</Eyebrow>
            <h2 id="safety-title">A pooled book must balance to the lamport.</h2>
            <p>
              Cinder enforces continuous mathematical invariants between the private rollup
              and public venue. If an invariant is violated, risk freezes automatically.
            </p>
          </div>

          <div className="guarantee-grid">
            <article className="guarantee-card">
              <div className="guarantee-badge">INVARIANT I1</div>
              <h3>Position Integrity</h3>
              <p className="formula">∑ User Positions = Book = Phoenix</p>
              <p>
                The sum of all filled private UserLedger positions must equal Cinder’s
                aggregate Book and match the position observed on Phoenix.
              </p>
            </article>

            <article className="guarantee-card">
              <div className="guarantee-badge">INVARIANT I2</div>
              <h3>Collateral Integrity</h3>
              <p className="formula">User Cash + PnL = Vault + Phoenix</p>
              <p>
                Private user cash balances and unrealized PnL reconcile with physical
                collateral deposited in the vault and equity held at Phoenix.
              </p>
            </article>

            <article className="guarantee-card halt-card">
              <div className="guarantee-badge halt">CIRCUIT BREAKER</div>
              <h3>Risk Stops on Mismatch</h3>
              <p className="formula">Drift Detected ⟹ Immediate HALT</p>
              <p>
                If an invariant condition breaks, Cinder halts new risk entries and order
                routing immediately rather than masking discrepancy or drift.
              </p>
            </article>
          </div>
        </section>

        {/* TECHNICAL FAQ ACCORDION */}
        <section className="section shell faq-section" id="faq" aria-labelledby="faq-title">
          <div className="section-intro centered-intro">
            <Eyebrow>TECHNICAL DEEP DIVE</Eyebrow>
            <h2 id="faq-title">Frequently Asked Questions</h2>
            <p>
              Understand how Cinder enforces privacy, coordinates execution, and maintains
              provable protocol solvency.
            </p>
          </div>

          <FaqAccordion />
        </section>

        {/* FINAL CALL TO ACTION */}
        <section className="final-cta" aria-labelledby="cta-title">
          <Image
            className="footer-art"
            src="/images/footer_cluster.png"
            alt=""
            width={1672}
            height={941}
          />
          <div className="shell final-shell">
            <div className="final-copy">
              <div className="final-badge">
                <span>✦ OPEN SOURCE EXPERIMENTAL PROTOTYPE</span>
              </div>
              <h2 id="cta-title">See private prime brokerage in action.</h2>
              <p>
                Explore the deterministic Cinder terminal walkthrough with live Phoenix market
                data streams—no wallet signature and no order routing required.
              </p>
              <div className="hero-actions">
                <Link className="button button-primary glow-button" href="/terminal">
                  Open Demo Terminal <span aria-hidden="true">→</span>
                </Link>
                <a
                  className="button button-quiet"
                  href="https://github.com/arnabnandikgp/cinder"
                  target="_blank"
                  rel="noreferrer"
                >
                  View Source on GitHub ↗
                </a>
              </div>
            </div>
          </div>
        </section>
      </main>

      {/* FOOTER */}
      <footer className="site-footer shell">
        <div className="footer-left">
          <Brand />
          <p className="footer-tagline">
            Experimental private perp prime broker for Phoenix on Solana. Powered by
            MagicBlock Private Ephemeral Rollups.
          </p>
        </div>
        <div className="footer-links">
          <div className="footer-col">
            <span className="footer-col-title">Navigation</span>
            <a href="#how-it-works">How It Works</a>
            <a href="#netting-engine">Netting Engine</a>
            <a href="#architecture">Architecture</a>
            <a href="#privacy">Privacy Matrix</a>
          </div>
          <div className="footer-col">
            <span className="footer-col-title">Protocol</span>
            <Link href="/terminal">Demo Terminal</Link>
            <a href="https://github.com/arnabnandikgp/cinder" target="_blank" rel="noreferrer">
              GitHub Repository
            </a>
            <a href="https://phoenix.trade" target="_blank" rel="noreferrer">
              Phoenix DEX ↗
            </a>
            <a href="https://magicblock.gg" target="_blank" rel="noreferrer">
              MagicBlock ↗
            </a>
          </div>
        </div>
        <div className="footer-bottom">
          <p>© {new Date().getFullYear()} Cinder Protocol. Prototype for testing and research.</p>
          <p className="footer-disclaimer">Not investment advice. No live order routing.</p>
        </div>
      </footer>
    </div>
  );
}
