//! Canonical private cash claims, not an equity proof or an escape Merkle tree.
use crate::ledger::confirmed_position;
use crate::rpc::{Result, RuntimeError};
use crate::runtime::{ledger_id, pda, validate_ledger};
use anchor_lang::AccountSerialize;
use cinder_common as cc;
use cinder_ledger::{Book, UserLedger};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Aggregate-only publication data. Private claim rows must remain in memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReserveSnapshot {
    pub root: [u8; 32],
    pub user_count: u32,
    pub total_free: u64,
    pub total_reserved: u64,
    pub total_bad_debt: u64,
    pub book_hash: [u8; 32],
}
impl ReserveSnapshot {
    pub fn signed_cash_total(&self) -> i128 {
        i128::from(self.total_free) + i128::from(self.total_reserved)
            - i128::from(self.total_bad_debt)
    }
}

/// Require a complete registry and settled private OIDs/epochs. The caller must
/// additionally prove fresh authoritative I1/I2 and snapshot coherence before
/// publication; this pure encoding helper cannot attest venue state or finality.
pub fn build_reserve_snapshot(
    book: &Book,
    ledgers: &[([u8; 32], UserLedger)],
    registry: &BTreeMap<[u8; 32], [u8; 32]>,
) -> Result<ReserveSnapshot> {
    if book.schema_version != cc::ACCOUNT_SCHEMA_VERSION
        || book.invariant_ok != 1
        || book.halt & cc::INVARIANT_BROKEN != 0
        || book.residual_len as usize > book.residuals.len()
    {
        return Err(RuntimeError::Identity);
    }
    let mut claims = BTreeMap::new();
    let mut confirmed = BTreeMap::new();
    let mut out = ReserveSnapshot {
        root: [0; 32],
        user_count: 0,
        total_free: 0,
        total_reserved: 0,
        total_bad_debt: 0,
        book_hash: [0; 32],
    };
    for (address, ledger) in ledgers {
        validate_ledger(ledger)?;
        // The money-edge outbox is not implemented yet. Do not encode an
        // outstanding withdrawal as if it were an absent cash-in-flight term.
        if ledger.withdrawable != 0 {
            return Err(RuntimeError::Unsupported);
        }
        let user = ledger.user.to_bytes();
        if *address != pda(ledger_id(), &[cc::SEED_USER, &user])
            || registry.get(address) != Some(&user)
            || claims.insert(user, ledger).is_some()
        {
            return Err(RuntimeError::Identity);
        }
        if ledger.pending_oid_count != 0 || ledger.last_funding_epoch != book.funding_epoch {
            return Err(RuntimeError::Incomplete);
        }
        for p in &ledger.positions[..ledger.positions_len as usize] {
            let lots = confirmed.entry(p.asset_id).or_insert(0i128);
            *lots = lots
                .checked_add(i128::from(confirmed_position(p, &ledger.open_oids)?))
                .ok_or(RuntimeError::Decode)?;
        }
        for (total, value) in [
            (&mut out.total_free, ledger.free),
            (&mut out.total_reserved, ledger.reserved),
            (&mut out.total_bad_debt, ledger.bad_debt_usdc),
        ] {
            *total = total.checked_add(value).ok_or(RuntimeError::Decode)?;
        }
    }
    if claims.len() != registry.len() {
        return Err(RuntimeError::Incomplete);
    }
    let mut residuals = BTreeMap::new();
    for r in &book.residuals[..book.residual_len as usize] {
        if r.asset_id == 0 || residuals.insert(r.asset_id, i128::from(r.lots)).is_some() {
            return Err(RuntimeError::Identity);
        }
    }
    confirmed.retain(|_, lots| *lots != 0);
    residuals.retain(|_, lots| *lots != 0);
    if confirmed != residuals {
        return Err(RuntimeError::Identity);
    }
    out.user_count = u32::try_from(claims.len()).map_err(|_| RuntimeError::Decode)?;
    let mut hash = Sha256::new();
    hash.update(b"cinder:cash-claims:v1");
    hash.update([cc::ACCOUNT_SCHEMA_VERSION]);
    hash.update(out.user_count.to_le_bytes());
    // Raw pubkey byte order, not base58 text order. Fixed-width LE integers
    // and an explicit version/count make the concatenation unambiguous.
    for (user, ledger) in claims {
        hash.update(user);
        for value in [ledger.free, ledger.reserved, ledger.bad_debt_usdc] {
            hash.update(value.to_le_bytes());
        }
        let mut positions = ledger.positions[..ledger.positions_len as usize]
            .iter()
            .map(|p| (p.asset_id, p.lots))
            .collect::<Vec<_>>();
        positions.sort_by_key(|(asset, _)| *asset);
        hash.update([ledger.positions_len]);
        for (asset, lots) in positions {
            hash.update(asset.to_le_bytes());
            hash.update(lots.to_le_bytes());
        }
    }
    out.root = hash.finalize().into();
    let mut bytes = Vec::with_capacity(Book::ACCOUNT_SPACE);
    book.try_serialize(&mut bytes)
        .map_err(|_| RuntimeError::Decode)?;
    let mut hash = Sha256::new();
    hash.update(b"cinder:book:v1");
    hash.update(bytes);
    out.book_hash = hash.finalize().into();
    Ok(out)
}
