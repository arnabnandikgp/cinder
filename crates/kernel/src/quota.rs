//! RF2 largest-remainder reduction and immutable earliest-deadline restoration.
//! Compilation has an explicit work/space budget. Repeated blocks are skipped
//! only after checking affine inequalities that certify the identical EDF choices.
//! Unsupported profiles fail closed; there is no alternative allocation policy.
use crate::identity::AccountId;
use alloc::{vec, vec::Vec};

/// Compiler/lookup failure, not an allocation that may be partially installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// Empty, duplicate, zero, excessive or inconsistent quantities.
    Invalid,
    /// Exact EDF cannot be represented within the configured finite envelope.
    Budget,
}
/// Canonical customer quantity. Zero rows are omitted after initial apportionment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weight {
    /// Immutable private identity; byte order breaks exact ties.
    pub owner: AccountId,
    /// Positive integer lots (or atoms for a separate fee apportionment).
    pub amount: u64,
}
const BLOCK: usize = 64;
const MAX_WORK: usize = 2_000_000;
const MAX_SEGMENTS: usize = 1024;
const MAX_MEMBERS: usize = 64;

fn canonical(input: &[Weight]) -> Result<(Vec<Weight>, u64), Failure> {
    if input.is_empty() || input.len() > MAX_MEMBERS {
        return Err(Failure::Invalid);
    }
    let mut rows = input.to_vec();
    rows.sort_by_key(|r| r.owner);
    let mut sum = 0u64;
    for (i, r) in rows.iter().enumerate() {
        if r.amount == 0 || i > 0 && rows[i - 1].owner == r.owner {
            return Err(Failure::Invalid);
        }
        sum = sum.checked_add(r.amount).ok_or(Failure::Invalid)?;
    }
    if sum > i64::MAX as u64 {
        return Err(Failure::Invalid);
    }
    Ok((rows, sum))
}
/// Largest remainders once at the original cut, never recalculated per fill.
/// The amount need not be <= total (this also apportions nonnegative fees).
pub fn apportion(input: &[Weight], amount: u64) -> Result<Vec<Weight>, Failure> {
    let (mut rows, total) = canonical(input)?;
    if amount > i64::MAX as u64 {
        return Err(Failure::Invalid);
    }
    let mut rem = Vec::with_capacity(rows.len());
    let mut allocated = 0;
    for (i, r) in rows.iter_mut().enumerate() {
        let n = u128::from(r.amount) * u128::from(amount);
        r.amount = (n / u128::from(total)) as u64;
        allocated += r.amount;
        rem.push((n % u128::from(total), i));
    }
    rem.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, i) in rem.into_iter().take((amount - allocated) as usize) {
        rows[i].amount += 1;
    }
    rows.retain(|r| r.amount > 0);
    Ok(rows)
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Segment {
    pattern: Vec<u8>,
    repeats: u64,
}
/// No public constructor/manifest import can bypass the canonical compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    weights: Vec<Weight>,
    total: u64,
    segments: Vec<Segment>,
    work: usize,
}
// Largest h>=0 through which x+h*d >= floor. None means false already at h=0.
fn at_least(x: i128, d: i128, floor: i128) -> Option<u64> {
    if x < floor {
        None
    } else if d >= 0 {
        Some(u64::MAX)
    } else {
        Some(((x - floor) / (-d)).min(i128::from(u64::MAX)) as u64)
    }
}
fn at_most(x: i128, d: i128, ceiling: i128) -> Option<u64> {
    at_least(-x, -d, -ceiling)
}
impl Plan {
    /// Compile the full immutable schedule or return a bounded-work failure.
    /// Positive quantities and their sum are bounded to i64::MAX; all affine
    /// products therefore fit i128. Runtime/space limits do not scale with lots.
    pub fn compile(input: &[Weight]) -> Result<Self, Failure> {
        let (weights, total) = canonical(input)?;
        let n = weights.len();
        fn gcd(mut a: u64, mut b: u64) -> u64 {
            while b != 0 {
                (a, b) = (b, a % b);
            }
            a
        }
        let divisor = weights.iter().fold(total, |a, w| gcd(a, w.amount));
        let period = total / divisor;
        let block = if period <= 4096 {
            period as usize
        } else {
            BLOCK
        };
        let mut done = vec![0u64; n];
        let mut slot = 1u64;
        let mut segments = Vec::new();
        let mut work = 0;
        let mut stored = 0;
        while slot <= total {
            let start = done.clone();
            let first = slot;
            let mut pattern = Vec::new();
            for _ in 0..block.min((total - slot + 1) as usize) {
                work += n;
                if work > MAX_WORK {
                    return Err(Failure::Budget);
                }
                let mut best = None;
                for (i, w) in weights.iter().enumerate() {
                    if done[i] == w.amount {
                        continue;
                    }
                    let release =
                        u128::from(done[i]) * u128::from(total) / u128::from(w.amount) + 1;
                    if release > u128::from(slot) {
                        continue;
                    }
                    let deadline = (u128::from(done[i] + 1) * u128::from(total))
                        .div_ceil(u128::from(w.amount));
                    if best.is_none_or(|(_, d)| deadline < d) {
                        best = Some((i, deadline));
                    }
                }
                let (winner, deadline) = best.ok_or(Failure::Invalid)?;
                if deadline < u128::from(slot) {
                    return Err(Failure::Invalid);
                }
                done[winner] += 1;
                pattern.push(winner as u8);
                slot += 1;
            }
            let len = pattern.len() as u64;
            let counts: Vec<u64> = done.iter().zip(&start).map(|(a, b)| a - b).collect();
            let drift: Vec<i128> = counts
                .iter()
                .zip(&weights)
                .map(|(c, w)| {
                    i128::from(*c) * i128::from(total) - i128::from(len) * i128::from(w.amount)
                })
                .collect();
            // h is the number of additional copies after the block just compiled.
            let mut hmax = (total - first + 1) / len - 1;
            for (i, w) in weights.iter().enumerate() {
                if let Some(copies) = (w.amount - start[i]).checked_div(counts[i]) {
                    hmax = hmax.min(copies - 1);
                }
            }
            let mut prefix = start.clone();
            for (offset, winner) in pattern.iter().enumerate() {
                if hmax == 0 {
                    break;
                }
                work += n;
                if work > MAX_WORK {
                    return Err(Failure::Budget);
                }
                let a = usize::from(*winner);
                let wa = i128::from(weights[a].amount);
                let time = i128::from(first + offset as u64);
                let prev = i128::from(prefix[a]) * i128::from(total);
                let next = prev + i128::from(total);
                let deadline = (next + wa - 1) / wa;
                // Winner is available, and its normalized deadline stays fixed.
                for bound in [
                    at_most(prev, drift[a], time * wa - 1),
                    at_least(next, drift[a], (deadline - 1) * wa + 1),
                    at_most(next, drift[a], deadline * wa),
                ] {
                    hmax = hmax.min(bound.ok_or(Failure::Invalid)?);
                }
                for (i, w) in weights.iter().enumerate() {
                    if i == a || prefix[i] == w.amount {
                        continue;
                    }
                    let wi = i128::from(w.amount);
                    let prev = i128::from(prefix[i]) * i128::from(total);
                    let unavailable = at_least(prev, drift[i], time * wi);
                    let threshold = if i < a {
                        deadline * wi + 1
                    } else {
                        (deadline - 1) * wi + 1
                    };
                    let later = at_least(prev + i128::from(total), drift[i], threshold);
                    // Either condition is enough; each holds on a contiguous
                    // prefix from h=0, so the union reaches the greater bound.
                    let safe = match (unavailable, later) {
                        (Some(x), Some(y)) => x.max(y),
                        (Some(x), None) | (None, Some(x)) => x,
                        (None, None) => return Err(Failure::Invalid),
                    };
                    hmax = hmax.min(safe);
                }
                prefix[a] += 1;
            }
            for i in 0..n {
                done[i] += counts[i] * hmax;
            }
            slot += len * hmax;
            stored += pattern.len();
            segments.push(Segment {
                pattern,
                repeats: hmax + 1,
            });
            if segments.len() > MAX_SEGMENTS || stored > 65536 {
                return Err(Failure::Budget);
            }
        }
        if done.iter().zip(&weights).any(|(d, w)| *d != w.amount) {
            return Err(Failure::Invalid);
        }
        Ok(Self {
            weights,
            total,
            segments,
            work,
        })
    }
    /// Canonical fixed membership/reduction quantities.
    pub fn weights(&self) -> &[Weight] {
        &self.weights
    }
    /// Total admitted reduction/restoration capacity.
    pub fn total(&self) -> u64 {
        self.total
    }
    /// Compiler work units, for qualification and regression evidence.
    pub fn work(&self) -> usize {
        self.work
    }
    /// Number of compressed segments, not the number of atomic lots.
    pub fn segments(&self) -> usize {
        self.segments.len()
    }
    /// Exact counts in [0,prefix), bounded by 65536 stored jobs, not prefix.
    pub fn prefix(&self, mut prefix: u64) -> Result<Vec<u64>, Failure> {
        if prefix > self.total {
            return Err(Failure::Invalid);
        }
        let mut counts = vec![0; self.weights.len()];
        for segment in &self.segments {
            let len = segment.pattern.len() as u64;
            let take = prefix.min(len * segment.repeats);
            let full = take / len;
            let partial = take % len;
            for (j, i) in segment.pattern.iter().enumerate() {
                counts[usize::from(*i)] += full + u64::from((j as u64) < partial);
            }
            prefix -= take;
            if prefix == 0 {
                break;
            }
        }
        Ok(counts)
    }
}
