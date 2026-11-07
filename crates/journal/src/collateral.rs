//! Shared marked-collateral prerequisite. P09 adds pending execution, stress
//! capital/concentration and selected-leverage policy; this is not live admission.
use crate::{
    Error,
    model::*,
    wire::{Reader, Writer},
};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{evidence::*, *},
    math::{Rounding, mul_div},
};

/// Explicit initial collateral parameters, not venue-derived defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarginRule {
    /// Versioned configured market.
    pub market: MarketUnit,
    /// Private initial margin in basis points, 1..=10,000.
    pub private_bps: u32,
    /// Native account initial margin in basis points, 1..=10,000.
    pub native_bps: u32,
}
/// Trusted risk-policy configuration. Tests use synthetic values only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// Immutable configuration revision; changed parameters require a higher one.
    pub revision: PolicyVersion,
    /// Complete per-market initial margins.
    pub markets: Vec<MarginRule>,
    /// Qualified evidence freshness and containment horizons.
    pub evidence: EvidencePolicy,
}
/// Authenticated-source prices evaluated at the current atomic ledger cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cut {
    /// Exact state revision at installation; not a caller-selected equity number.
    pub expected_version: u64,
    /// Explicit trusted policy. A public API must not accept arbitrary policies.
    pub policy: Policy,
    /// Qualified price observations; source authenticity is P13's port obligation.
    pub marks: Vec<MarkObservation>,
}

impl State {
    pub(crate) fn install_collateral(&mut self, cut: &Cut) -> Result<(), ControlError> {
        if cut.expected_version != self.ledger.version()
            || cut.policy.markets.len() != self.config.markets.len()
            || self.collateral.as_ref().is_some_and(|old| {
                cut.policy.revision < old.policy.revision
                    || cut.policy.revision == old.policy.revision && cut.policy != old.policy
            })
            || cut.policy.markets.iter().enumerate().any(|(i, r)| {
                r.private_bps == 0
                    || r.private_bps > 10_000
                    || r.native_bps == 0
                    || r.native_bps > 10_000
                    || !self.config.markets.iter().any(|m| m.unit() == r.market)
                    || cut.policy.markets[..i]
                        .iter()
                        .any(|old| old.market == r.market)
            })
        {
            return Err(ControlError::Invalid);
        }
        self.ledger
            .qualified_diagnostics(&cut.marks, self.now, cut.policy.evidence)
            .map_err(|_| ControlError::Unqualified)?;
        self.collateral = Some(cut.clone());
        Ok(())
    }

    pub(crate) fn collateral_capacity(
        &self,
        resource: Resource,
    ) -> Result<QuoteAtoms, ControlError> {
        let c = self.collateral.as_ref().ok_or(ControlError::Unqualified)?;
        let diagnostic = self
            .ledger
            .qualified_diagnostics(&c.marks, self.now, c.policy.evidence)
            .map_err(|_| ControlError::Unqualified)?;
        if diagnostic.shortfall.atoms() > 0
            || diagnostic.unresolved_events > 0
            || self.orders.iter().any(|o| {
                o.faulted
                    || o.remaining() > 0
                        && self
                            .holds
                            .iter()
                            .any(|h| h.request == o.intent.request && h.active)
            })
        {
            // Pending-order risk is not assumed zero; P09 evaluates its outcome set.
            return Err(ControlError::Unqualified);
        }
        if resource == Resource::Location(Location::Vault) {
            return Ok(self.ledger.vault());
        }
        if resource == Resource::Location(Location::Broker) {
            return Ok(self.ledger.broker());
        }
        let b = match resource {
            Resource::Customer(id) => self.ledger.book(Owner::Customer(id)),
            Resource::House => self.ledger.book(Owner::House),
            Resource::Location(_) => Ok(self.ledger.venue()),
        }
        .map_err(|_| ControlError::Invalid)?;
        let mut free = b
            .cash()
            .atoms()
            .checked_add(b.funding().atoms().min(0))
            .ok_or(ControlError::Capacity)?;
        for p in b.positions() {
            let market = self
                .config
                .markets
                .iter()
                .find(|m| m.unit() == p.quantity().unit())
                .ok_or(ControlError::Invalid)?;
            let mark = c
                .marks
                .iter()
                .find(|m| m.price.unit() == market.unit())
                .ok_or(ControlError::Unqualified)?
                .price;
            let rule = c
                .policy
                .markets
                .iter()
                .find(|r| r.market == market.unit())
                .ok_or(ControlError::Unqualified)?;
            let bps = if resource == Resource::Location(Location::Venue) {
                rule.native_bps
            } else {
                rule.private_bps
            };
            let notional = market
                .notional(p.quantity(), mark)
                .map_err(|_| ControlError::Capacity)?
                .atoms()
                .checked_abs()
                .ok_or(ControlError::Capacity)?;
            let margin = mul_div(notional, u64::from(bps), 10_000, Rounding::Ceil)
                .map_err(|_| ControlError::Capacity)?;
            free = free
                .checked_add(
                    p.unrealized(*market, mark)
                        .map_err(|_| ControlError::Capacity)?
                        .atoms(),
                )
                .and_then(|n| n.checked_sub(margin))
                .ok_or(ControlError::Capacity)?;
        }
        if resource == Resource::Location(Location::Venue) {
            free = free.min(b.cash().atoms());
        }
        if resource == Resource::House {
            // A nominal transit receivable is not eligible loss-bearing cash.
            let eligible = diagnostic
                .backing_margin
                .checked_sub(
                    self.ledger
                        .in_transit()
                        .map_err(|_| ControlError::Unqualified)?,
                )
                .map_err(|_| ControlError::Capacity)?;
            free = free.min(eligible.atoms());
        }
        Ok(QuoteAtoms::new(self.config.quote, free.max(0)))
    }
}

pub(crate) fn encode(w: &mut Writer, c: &Cut) {
    w.u64(c.expected_version);
    w.raw(&c.policy.revision.get().to_be_bytes());
    w.count(c.policy.markets.len());
    for r in &c.policy.markets {
        w.item(&QuantityLots::new(r.market, 0));
        w.raw(&r.private_bps.to_be_bytes());
        w.raw(&r.native_bps.to_be_bytes());
    }
    w.u64(c.policy.evidence.max_issue_age);
    w.u64(c.policy.evidence.max_mark_age);
    w.u64(c.policy.evidence.max_check_age);
    w.count(c.marks.len());
    for m in &c.marks {
        w.item(&m.price);
        w.item(&m.evidence);
        w.u64(m.observed_at);
        w.u64(m.valid_until);
        w.byte(u8::from(m.qualified));
    }
}
pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Cut, Error> {
    let expected_version = r.u64()?;
    let revision = PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)?;
    let mut markets = vec![];
    for _ in 0..r.count()? {
        let q: QuantityLots = r.item()?;
        if q.lots() != 0 {
            return Err(Error::Codec);
        }
        markets.push(MarginRule {
            market: q.unit(),
            private_bps: u32::from_be_bytes(r.array()?),
            native_bps: u32::from_be_bytes(r.array()?),
        });
    }
    let evidence = EvidencePolicy {
        max_issue_age: r.u64()?,
        max_mark_age: r.u64()?,
        max_check_age: r.u64()?,
    };
    let mut marks = vec![];
    for _ in 0..r.count()? {
        marks.push(MarkObservation {
            price: r.item()?,
            evidence: r.item()?,
            observed_at: r.u64()?,
            valid_until: r.u64()?,
            qualified: crate::orders::read_bool(r)?,
        });
    }
    Ok(Cut {
        expected_version,
        policy: Policy {
            revision,
            markets,
            evidence,
        },
        marks,
    })
}
