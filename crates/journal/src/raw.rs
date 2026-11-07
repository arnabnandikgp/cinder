//! Qualified resolution of one retained raw input, never an administrative reset.
//! Source authentication and completeness are trusted-port obligations. An opaque
//! evidence reference is not itself a cryptographic proof or native finality.
use crate::{
    Error,
    model::*,
    wire::{MAX_ITEMS, Reader, Writer},
};
use cinder_kernel::{
    identity::{EventScope, RecordKey},
    ledger::evidence::Disposition,
};
use sha2::{Digest, Sha256};
use std::fmt;

/// Stable input identity in the immutable journal, scoped by its genesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputKey {
    /// Original accepted transaction, not a venue event identity.
    pub transaction: CommitId,
    /// Zero-based input ordinal in that exact transaction.
    pub ordinal: u64,
}
/// Retained containment record. It owns no assets and cannot hide lifecycle faults.
#[derive(Clone, PartialEq, Eq)]
pub struct Entry {
    /// Original transaction/input identity.
    pub key: InputKey,
    /// SHA-256 of the full canonical original Input, including its envelope.
    pub fingerprint: [u8; 32],
    /// Original configured or rejected source; resolution must match it exactly.
    pub source: EventScope,
    /// Original qualification revision, never silently rebound.
    pub authority_epoch: u64,
    /// Original source-local causal cut, if one was supplied.
    pub source_cut: Option<u64>,
    /// Original observed time, not a source-global sequence.
    pub observed_at: u64,
    /// Only valid unnormalized envelopes can receive a no-effect finding.
    pub no_effect_allowed: bool,
    /// Permanent exact resolution digest; same target cannot be reinterpreted.
    pub resolved: Option<[u8; 32]>,
}
impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RawEntry([PRIVATE])")
    }
}
/// A qualified source's resolution of one exact input. This is a trusted internal
/// port result, not a customer instruction, bearer grant or proof verifier.
#[derive(Clone, PartialEq, Eq)]
pub struct Resolution {
    /// Exact original retained input.
    pub key: InputKey,
    /// Must match the original canonical Input fingerprint.
    pub fingerprint: [u8; 32],
    /// Independently authenticated source; must match the original/configuration.
    pub source: EventScope,
    /// Qualified source revision, matching the retained input.
    pub authority_epoch: u64,
    /// Source-local complete coverage established by the qualification port.
    pub through: u64,
    /// Qualified observation time in the journal's injected time domain.
    pub observed_at: u64,
    /// Nonzero commitment to retained source qualification evidence.
    pub evidence: [u8; 32],
    /// Explicit authenticated finding that this input has no economic effect.
    /// A timeout, empty page, matching balance or successful parse is not enough.
    pub no_effect: bool,
    /// Complete effects linked by the qualification port. Each must already be
    /// applied in this ledger (possibly in this same transaction), never hypothetical.
    pub effects: Vec<cinder_kernel::identity::EventKey>,
}
impl fmt::Debug for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RawResolution([PRIVATE])")
    }
}
impl Resolution {
    /// Immutable digest binding evidence, coverage and exact effect set.
    pub fn digest(&self) -> Result<[u8; 32], Error> {
        let mut w = Writer::new(78);
        encode_resolution(&mut w, self);
        Ok(Sha256::digest(w.finish()?).into())
    }
}
impl State {
    /// Pending and resolved raw-input records; no independent financial projection.
    pub fn raw_inputs(&self) -> &[Entry] {
        &self.raw_inputs
    }
    pub(crate) fn retain_raw(
        &mut self,
        key: InputKey,
        input: &Input,
        valid: bool,
    ) -> Result<(), Error> {
        if self.raw_inputs.len() >= MAX_ITEMS || self.raw_inputs.iter().any(|e| e.key == key) {
            return Err(Error::Limit);
        }
        self.raw_inputs.push(Entry {
            key,
            fingerprint: crate::wire::input_fingerprint(input)?,
            source: input.source,
            authority_epoch: input.authority_epoch,
            source_cut: input.source_cut,
            observed_at: input.observed_at,
            no_effect_allowed: valid && input.event.is_none(),
            resolved: None,
        });
        Ok(())
    }
    pub(crate) fn resolve_raw(&mut self, r: &Resolution) -> Result<(), ControlError> {
        let index = self
            .raw_inputs
            .iter()
            .position(|e| e.key == r.key)
            .ok_or(ControlError::Invalid)?;
        let original = &self.raw_inputs[index];
        let digest = r.digest().map_err(|_| ControlError::Invalid)?;
        if let Some(old) = original.resolved {
            return if old == digest {
                Ok(())
            } else {
                Err(ControlError::Invalid)
            };
        }
        if r.fingerprint != original.fingerprint
            || r.source != original.source
            || r.source.domain != self.config.domain
            || !self.config.sources.iter().any(|s| s.scope == r.source)
            || r.authority_epoch == 0
            || r.authority_epoch != original.authority_epoch
            || r.evidence == [0; 32]
            || r.through == 0
            || original.source_cut.is_some_and(|cut| cut > r.through)
            || r.observed_at < original.observed_at
            || r.observed_at > self.now
            || r.effects.len() > MAX_ITEMS
            || r.no_effect != r.effects.is_empty()
            || (r.no_effect && !original.no_effect_allowed)
            || r.effects.iter().enumerate().any(|(i, key)| {
                key.scope != r.source
                    || r.effects[..i].contains(key)
                    || !self.ledger.observations().iter().any(|o| {
                        o.event.key == RecordKey::Economic(key.clone())
                            && o.disposition == Disposition::Applied
                    })
            })
        {
            return Err(ControlError::Unqualified);
        }
        self.raw_unresolved = self
            .raw_unresolved
            .checked_sub(1)
            .ok_or(ControlError::Invalid)?;
        self.raw_inputs[index].resolved = Some(digest);
        Ok(())
    }
}
fn key(w: &mut Writer, key: InputKey) {
    w.raw(&key.transaction.bytes());
    w.u64(key.ordinal);
}
fn read_key(r: &mut Reader<'_>) -> Result<InputKey, Error> {
    Ok(InputKey {
        transaction: CommitId::new(r.array()?)?,
        ordinal: r.u64()?,
    })
}
pub(crate) fn encode_resolution(w: &mut Writer, r: &Resolution) {
    key(w, r.key);
    w.raw(&r.fingerprint);
    crate::wire::scope(w, r.source);
    w.u64(r.authority_epoch);
    w.u64(r.through);
    w.u64(r.observed_at);
    w.raw(&r.evidence);
    w.byte(u8::from(r.no_effect));
    w.count(r.effects.len());
    for e in &r.effects {
        w.item(e);
    }
}
pub(crate) fn decode_resolution(r: &mut Reader<'_>) -> Result<Resolution, Error> {
    let key = read_key(r)?;
    let fingerprint = r.array()?;
    let source = crate::wire::read_scope(r)?;
    let authority_epoch = r.u64()?;
    let through = r.u64()?;
    let observed_at = r.u64()?;
    let evidence = r.array()?;
    let no_effect = r.bool()?;
    let mut effects = vec![];
    for _ in 0..r.count()? {
        effects.push(r.item()?);
    }
    Ok(Resolution {
        key,
        fingerprint,
        source,
        authority_epoch,
        through,
        observed_at,
        evidence,
        no_effect,
        effects,
    })
}
pub(crate) fn encode_entries(w: &mut Writer, entries: &[Entry]) {
    w.count(entries.len());
    for e in entries {
        key(w, e.key);
        w.raw(&e.fingerprint);
        crate::wire::scope(w, e.source);
        w.u64(e.authority_epoch);
        w.option(&e.source_cut, |w, cut| w.u64(*cut));
        w.u64(e.observed_at);
        w.byte(u8::from(e.no_effect_allowed));
        w.option(&e.resolved, |w, digest| w.raw(digest));
    }
}
