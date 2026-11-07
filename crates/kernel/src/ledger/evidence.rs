//! Explicit incomplete evidence, observation retention and age/freshness gates.
//! Time is injected monotone milliseconds, never read from a clock. No source is
//! authenticated by constructing these structs; P13 owns native qualification.

use super::*;

/// Reason for a named, retained reconciliation condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    /// Actual PnL report differs from the exact native position calculation.
    NativePnl,
    /// Rate or native amount at a frozen funding boundary remains unknown.
    FundingInputs,
    /// Actual native funding differs from the qualified rule, not just rounding.
    FundingDifference,
    /// Incomplete or mismatching native snapshot; no balancing cash is invented.
    NativeSnapshot,
    /// Normalized fact cannot yet be applied; its payload remains available.
    Rejected(LedgerError),
    /// Same semantic key but changed economics, including a previously rejected input.
    ReplayConflict,
}

/// A named condition; its first-seen time is not reset by duplicate/repeated checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Economic/operation subject.
    pub subject: RecordKey,
    /// Distinct reason; one event may have more than one condition.
    pub kind: IssueKind,
    /// First detection in this unresolved episode, injected milliseconds.
    pub first_seen: u64,
    /// Last changed observation, not a new age origin.
    pub last_seen: u64,
    /// A known signed difference; None means not measured, never zero.
    pub difference: Option<QuoteAtoms>,
    /// Historical resolved conditions remain visible.
    pub open: bool,
}

/// Ingestion disposition, separate from the immutable economic payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Exactly one successful financial/metadata transition.
    Applied,
    /// Same economic input, no repeated posting.
    Duplicate,
    /// Retained input and explicit containment, without partial financial changes.
    Rejected(LedgerError),
}

/// Retained normalized observation. This is not a raw wire archive or durable journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// Semantic event; arrival metadata is not part of deduplication.
    pub event: Event,
    /// Trusted injected arrival time, separate from native execution order.
    pub observed_at: u64,
    /// Reproducible outcome.
    pub disposition: Disposition,
}

/// Proposed result of ingestion; caller must durably commit it before acknowledging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ingested {
    /// Same authoritative ledger, including retained rejected evidence.
    pub state: Ledger,
    /// No transport success/failure is inferred from this classification.
    pub disposition: Disposition,
}

/// Complete source-qualified native components at an exact local projection cut.
/// None is unknown. Positions must explicitly include flat configured markets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCheck {
    /// Local projection version covered by the source reconciliation.
    pub expected_version: u64,
    /// Actual native signed cash, not total equity.
    pub cash: Option<QuoteAtoms>,
    /// Actual recognized unsettled funding.
    pub funding: Option<QuoteAtoms>,
    /// Exact native quantities and basis, complete across configured markets.
    pub positions: Option<Vec<Position>>,
    /// Qualified source/history completeness, not simply end-of-pagination.
    pub complete: bool,
    /// Explicit older checks to resolve; a fresh snapshot never resets all issues.
    pub resolves: Vec<CheckResolution>,
}

/// Evidence-backed resolution of an older snapshot condition, never a balance write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResolution {
    /// Earlier NativeCheck event.
    pub check: EventKey,
    /// Unique intervening native effects must explain every originally known
    /// component exactly. Unknown components can be completed without postings.
    pub applied_effects: Vec<EventKey>,
}

/// Explicit policy thresholds, all in injected milliseconds. Synthetic values in
/// tests are not calibrated live freshness or availability recommendations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidencePolicy {
    /// Maximum mark observation age.
    pub max_mark_age: u64,
    /// Unresolved conditions escalate from restricted to frozen at this age.
    pub max_issue_age: u64,
    /// Maximum age of a matching complete native reconciliation.
    pub max_check_age: u64,
}

/// Financial evidence mode only; Reconciled does not mean margin/capital approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceMode {
    /// Evidence prerequisites satisfied; full P09 risk checks still required.
    Reconciled,
    /// Missing/inconsistent evidence blocks dependent risk/payout decisions.
    Restricted,
    /// Conflict or aged discrepancy requires stronger containment/reconciliation.
    Frozen,
}

/// Qualified mark envelope; a timestamp alone does not qualify a price source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkObservation {
    /// Versioned exact price ticks.
    pub price: PriceTicks,
    /// Configured source and economic observation identity.
    pub evidence: EventKey,
    /// Qualified source timestamp in the configured observation-time domain.
    pub observed_at: u64,
    /// Source/policy expiry, inclusive.
    pub valid_until: u64,
    /// False for uncertain, corrupt, wide-confidence or nontrading source data.
    pub qualified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct EvidenceState {
    now: u64,
    issues: Vec<Issue>,
    observations: Vec<Observation>,
    last_matching_check: Option<u64>,
    pub(super) effects: Vec<NativeEffect>,
    checks: Vec<(EventKey, Book)>,
}

/// Derived audit deltas, not another asset book. Positions use configured market
/// order; wider lot differences can represent an i64 extreme-to-extreme change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeDelta {
    cash: QuoteAtoms,
    funding: QuoteAtoms,
    positions: Vec<(i128, i128)>,
}
impl NativeDelta {
    pub(super) fn between(before: &Book, after: &Book) -> Result<Self, LedgerError> {
        Ok(Self {
            cash: after.cash.checked_sub(before.cash)?,
            funding: after.funding.checked_sub(before.funding)?,
            positions: before
                .positions
                .iter()
                .zip(&after.positions)
                .map(|(b, a)| {
                    Ok((
                        i128::from(a.quantity().lots()) - i128::from(b.quantity().lots()),
                        a.basis()
                            .atoms()
                            .checked_sub(b.basis().atoms())
                            .ok_or(Error::Overflow)?,
                    ))
                })
                .collect::<Result<_, LedgerError>>()?,
        })
    }
    fn add(&mut self, other: &Self) -> Result<(), LedgerError> {
        self.cash = self.cash.checked_add(other.cash)?;
        self.funding = self.funding.checked_add(other.funding)?;
        for ((q, b), (dq, db)) in self.positions.iter_mut().zip(&other.positions) {
            *q = q.checked_add(*dq).ok_or(Error::Overflow)?;
            *b = b.checked_add(*db).ok_or(Error::Overflow)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeEffect {
    pub(super) key: EventKey,
    // Preserve extremes without requiring an unrepresentable signed delta merely
    // to ingest them. Explanation arithmetic, if out of range, fails closed later.
    pub(super) before: Book,
    pub(super) after: Book,
}

impl Ledger {
    /// Preferred ingestion path: retain rejected/conflicting normalized facts and
    /// their arrival times, while keeping economic equality independent of transport.
    /// Input provenance must be authenticated before this trusted internal API.
    pub fn ingest(&self, event: &Event, observed_at: u64) -> Result<Ingested, LedgerError> {
        if observed_at < self.evidence.now {
            return Err(LedgerError::Evidence);
        }
        // Cross-deployment garbage is not allowed to contaminate another ledger.
        event.key.require_domain(self.config.domain)?;
        let mut base = self.clone();
        base.evidence.now = observed_at;
        let prior_conflict = self
            .evidence
            .observations
            .iter()
            .find(|o| o.event.key == event.key)
            .is_some_and(|o| o.event != *event);
        let duplicate = self.events.contains(event);
        let result = if prior_conflict {
            Err(LedgerError::Conflict)
        } else {
            base.apply(event)
        };
        let (mut next, disposition) = match result {
            Ok(mut state) => {
                // Exact retry can resolve a transient missing dependency; never
                // clears conflicting payload evidence or unrelated conditions.
                for issue in &mut state.evidence.issues {
                    if issue.subject == event.key && matches!(issue.kind, IssueKind::Rejected(_)) {
                        issue.open = false;
                        issue.last_seen = observed_at;
                    }
                }
                (
                    state,
                    if duplicate {
                        Disposition::Duplicate
                    } else {
                        Disposition::Applied
                    },
                )
            }
            Err(error) => {
                let kind = if error == LedgerError::Conflict {
                    IssueKind::ReplayConflict
                } else {
                    IssueKind::Rejected(error)
                };
                base.set_issue(event.key.clone(), kind, true, None);
                (base, Disposition::Rejected(error))
            }
        };
        // Every observed record gets one proposal version, including a contained
        // rejection or duplicate. apply() remains the metadata-free algebra seam.
        next.version = self.version.checked_add(1).ok_or(Error::Overflow)?;
        next.evidence.observations.push(Observation {
            event: event.clone(),
            observed_at,
            disposition,
        });
        Ok(Ingested {
            state: next,
            disposition,
        })
    }
    /// Retained observation history, including conflicting and unapplied inputs.
    pub fn observations(&self) -> &[Observation] {
        &self.evidence.observations
    }
    /// Read-only issue history; there is no generic admin reset or balancing setter.
    pub fn issues(&self) -> &[Issue] {
        &self.evidence.issues
    }
    pub(super) fn set_issue(
        &mut self,
        subject: RecordKey,
        kind: IssueKind,
        open: bool,
        difference: Option<QuoteAtoms>,
    ) {
        if let Some(issue) = self
            .evidence
            .issues
            .iter_mut()
            .find(|i| i.subject == subject && i.kind == kind)
        {
            if !issue.open && open {
                issue.first_seen = self.evidence.now;
            }
            issue.open = open;
            issue.last_seen = self.evidence.now;
            issue.difference = difference;
        } else if open {
            self.evidence.issues.push(Issue {
                subject,
                kind,
                first_seen: self.evidence.now,
                last_seen: self.evidence.now,
                difference,
                open,
            });
        }
    }
    pub(super) fn apply_check(
        &mut self,
        key: &RecordKey,
        check: &NativeCheck,
    ) -> Result<(), LedgerError> {
        let source = self.source(key, Location::Venue)?;
        if check.expected_version != self.version {
            return Err(LedgerError::StaleCut);
        }
        if let Some(cash) = check.cash {
            cash.unit().require(self.config.quote)?;
        }
        if let Some(funding) = check.funding {
            funding.unit().require(self.config.quote)?;
        }
        let positions_match = if let Some(positions) = &check.positions {
            let mut seen = Vec::new();
            let mut matched = positions.len() == self.config.markets.len();
            for position in positions {
                let index = self.market_index(position.quantity().unit())?;
                if seen.contains(&index) {
                    return Err(LedgerError::Valuation);
                }
                seen.push(index);
                matched &= self.venue.positions[index] == *position;
            }
            matched
        } else {
            false
        };
        let matched = check.complete
            && check.cash == Some(self.venue.cash)
            && check.funding == Some(self.venue.funding)
            && positions_match;
        if !matched && !check.resolves.is_empty() {
            return Err(LedgerError::Evidence);
        }
        for resolution in &check.resolves {
            let subject = RecordKey::Economic(resolution.check.clone());
            let original_index = self
                .events
                .iter()
                .position(|e| e.key == subject)
                .ok_or(LedgerError::UnknownIdentity)?;
            let Change::Reconcile(original) = &self.events[original_index].change else {
                return Err(LedgerError::Evidence);
            };
            if !self
                .evidence
                .issues
                .iter()
                .any(|i| i.subject == subject && i.kind == IssueKind::NativeSnapshot && i.open)
            {
                return Err(LedgerError::Evidence);
            }
            let projected = &self
                .evidence
                .checks
                .iter()
                .find(|(k, _)| *k == resolution.check)
                .ok_or(LedgerError::Evidence)?
                .1;
            let mut delta = NativeDelta::between(projected, projected)?;
            let mut seen_effects = Vec::new();
            for effect in &resolution.applied_effects {
                if seen_effects.contains(effect) {
                    return Err(LedgerError::Evidence);
                }
                seen_effects.push(effect.clone());
                let effect_key = RecordKey::Economic(effect.clone());
                let effect_index = self
                    .events
                    .iter()
                    .position(|e| e.key == effect_key)
                    .ok_or(LedgerError::UnknownIdentity)?;
                if effect_index <= original_index {
                    return Err(LedgerError::Evidence);
                }
                let native_effect = self
                    .evidence
                    .effects
                    .iter()
                    .find(|e| e.key == *effect)
                    .ok_or(LedgerError::Evidence)?;
                delta.add(&NativeDelta::between(
                    &native_effect.before,
                    &native_effect.after,
                )?)?;
            }
            // A fresh matching snapshot plus an arbitrary intervening event must
            // not erase an older discrepancy. Explain the original cut, not today.
            if original
                .cash
                .is_some_and(|v| projected.cash.checked_add(delta.cash) != Ok(v))
                || original
                    .funding
                    .is_some_and(|v| projected.funding.checked_add(delta.funding) != Ok(v))
            {
                return Err(LedgerError::Evidence);
            }
            if let Some(positions) = &original.positions {
                for p in positions {
                    let index = self.market_index(p.quantity().unit())?;
                    let before = projected.positions[index];
                    let (dq, db) = delta.positions[index];
                    if i128::from(before.quantity().lots()).checked_add(dq)
                        != Some(i128::from(p.quantity().lots()))
                        || before.basis().atoms().checked_add(db) != Some(p.basis().atoms())
                    {
                        return Err(LedgerError::Evidence);
                    }
                }
            }
            self.set_issue(subject, IssueKind::NativeSnapshot, false, None);
        }
        self.set_issue(
            key.clone(),
            IssueKind::NativeSnapshot,
            !matched,
            check
                .cash
                .map(|value| value.checked_sub(self.venue.cash))
                .transpose()?,
        );
        if matched {
            self.evidence.last_matching_check = Some(self.evidence.now);
        }
        self.evidence.checks.push((source, self.venue.clone()));
        Ok(())
    }
    /// Evidence-only state. A frozen/restricted result never suppresses ingestion
    /// of an actual fill, fee, funding payment or source correction.
    pub fn evidence_mode(
        &self,
        now: u64,
        policy: EvidencePolicy,
    ) -> Result<EvidenceMode, LedgerError> {
        if now < self.evidence.now
            || policy.max_issue_age == 0
            || policy.max_mark_age == 0
            || policy.max_check_age == 0
        {
            return Err(LedgerError::Evidence);
        }
        let mut restricted = !self.unresolved.is_empty()
            || self.unresolved_funds() != 0
            || self
                .evidence
                .last_matching_check
                .is_none_or(|at| now - at > policy.max_check_age);
        for issue in self.evidence.issues.iter().filter(|i| i.open) {
            if issue.kind == IssueKind::ReplayConflict
                || now - issue.first_seen >= policy.max_issue_age
            {
                return Ok(EvidenceMode::Frozen);
            }
            restricted = true;
        }
        Ok(if restricted {
            EvidenceMode::Restricted
        } else {
            EvidenceMode::Reconciled
        })
    }
    /// Obtain diagnostics only when evidence and all marks qualify. This blocks
    /// dependent use of stale/unknown data; it does not replace P09 admission.
    pub fn qualified_diagnostics(
        &self,
        marks: &[MarkObservation],
        now: u64,
        policy: EvidencePolicy,
    ) -> Result<Diagnostics, LedgerError> {
        if self.evidence_mode(now, policy)? != EvidenceMode::Reconciled {
            return Err(LedgerError::Restricted);
        }
        for mark in marks {
            self.source(&RecordKey::Economic(mark.evidence.clone()), Location::Venue)?;
            if !mark.qualified
                || mark.observed_at > now
                || mark.valid_until < now
                || mark.valid_until < mark.observed_at
                || now - mark.observed_at > policy.max_mark_age
            {
                return Err(LedgerError::Evidence);
            }
        }
        self.diagnostics(&marks.iter().map(|m| m.price).collect::<Vec<_>>())
    }
}
