//! Differential qualification of the bounded RF2 compiler against per-lot EDF.
use cinder_kernel::{identity::AccountId, quota::*};
fn rows(weights: &[u64]) -> Vec<Weight> {
    weights
        .iter()
        .enumerate()
        .map(|(i, w)| Weight {
            owner: AccountId::new([i as u8 + 1; 32]).unwrap(),
            amount: *w,
        })
        .collect()
}
fn naive(weights: &[u64]) -> Vec<Vec<u64>> {
    let total: u64 = weights.iter().sum();
    let mut done = vec![0; weights.len()];
    let mut all = vec![done.clone()];
    for slot in 1..=total {
        let i = (0..weights.len())
            .filter(|i| done[*i] < weights[*i] && done[*i] * total / weights[*i] < slot)
            .min_by_key(|i| (((done[*i] + 1) * total).div_ceil(weights[*i]), *i))
            .unwrap();
        done[i] += 1;
        for (j, w) in weights.iter().enumerate() {
            assert!(done[j] >= slot * w / total && done[j] <= (slot * w).div_ceil(total));
        }
        all.push(done.clone());
    }
    all
}
fn verify(weights: &[u64]) {
    let plan = Plan::compile(&rows(weights)).unwrap();
    for (n, expected) in naive(weights).iter().enumerate() {
        assert_eq!(
            &plan.prefix(n as u64).unwrap(),
            expected,
            "{weights:?} at {n}"
        );
    }
}
#[test]
fn all_3905_small_vectors_match_exact_edf_every_prefix() {
    fn walk(ws: &mut Vec<u64>, left: usize, count: &mut usize) {
        if left == 0 {
            verify(ws);
            *count += 1;
            return;
        }
        for w in 1..=5 {
            ws.push(w);
            walk(ws, left - 1, count);
            ws.pop();
        }
    }
    let mut count = 0;
    for n in 1..=5 {
        walk(&mut vec![], n, &mut count);
    }
    assert_eq!(count, 3905);
}
#[test]
fn seeded_uneven_skew_and_alabama_vectors_preserve_schedule() {
    let mut seed = 731u32;
    let mut rand = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };
    for _ in 0..250 {
        let n = 2 + rand() % 10;
        verify(
            &(0..n)
                .map(|_| 1 + u64::from(rand() % 80))
                .collect::<Vec<_>>(),
        );
    }
    for ws in [
        &[9900, 25, 25, 25, 25][..],
        &[60, 10, 10, 10, 10],
        &[1500, 1500, 900, 500, 500, 200],
    ] {
        verify(ws);
    }
}
#[test]
fn trillion_lot_profiles_have_bounded_compilation_and_exact_prefix_quotas() {
    for ws in [
        vec![1_000_000_000_000; 2],
        vec![1_000_000_000_003, 1_000_000_000_000],
        vec![
            990_000_000_000,
            2_500_000_000,
            2_500_000_000,
            2_500_000_000,
            2_500_000_000,
        ],
        vec![1_000_000_000_000],
    ] {
        let p = Plan::compile(&rows(&ws)).unwrap();
        assert!(p.work() <= 2_000_000 && p.segments() <= 1024);
        for n in [0, 1, 5, 64, 12345, p.total() / 2, p.total() - 1, p.total()] {
            let counts = p.prefix(n).unwrap();
            assert_eq!(counts.iter().sum::<u64>(), n);
            for (i, w) in ws.iter().enumerate() {
                let product = u128::from(n) * u128::from(*w);
                assert!(u128::from(counts[i]) >= product / u128::from(p.total()));
                assert!(u128::from(counts[i]) <= product.div_ceil(u128::from(p.total())));
            }
        }
    }
}
#[test]
fn reduction_plus_restore_error_is_below_two_lots() {
    for a in 1..=7 {
        for b in 1..=7 {
            for d in 1..=a + b {
                let input = rows(&[a, b]);
                let allocation = apportion(&input, d).unwrap();
                let p = Plan::compile(&allocation).unwrap();
                for n in 0..=d {
                    let restored = p.prefix(n).unwrap();
                    for row in &input {
                        let i = allocation.iter().position(|r| r.owner == row.owner);
                        let remaining = i.map(|i| allocation[i].amount - restored[i]).unwrap_or(0);
                        assert!(
                            (i128::from(remaining * (a + b)) - i128::from((d - n) * row.amount))
                                .abs()
                                < i128::from(2 * (a + b))
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn canonical_input_precision_splitting_and_invalid_profiles() {
    let rs = rows(&[7, 3, 2]);
    let mut reverse = rs.clone();
    reverse.reverse();
    assert_eq!(Plan::compile(&rs), Plan::compile(&reverse));
    assert_eq!(
        apportion(
            &rows(&[4_000_000_000_000_000_001, 4_000_000_000_000_000_000]),
            3
        )
        .unwrap()
        .iter()
        .map(|r| r.amount)
        .collect::<Vec<_>>(),
        [2, 1]
    );
    // Splitting a later identity creates a tie that changes the winner.
    assert_eq!(
        apportion(&rows(&[1, 2]), 1).unwrap()[0].owner,
        rows(&[1, 2])[1].owner
    );
    assert_eq!(
        apportion(&rows(&[1, 1, 1]), 1).unwrap()[0].owner,
        rows(&[1])[0].owner
    );
    for rs in [
        vec![],
        rows(&[0]),
        rows(&[u64::MAX]),
        vec![rows(&[1])[0]; 2],
        rows(&[1; 65]),
    ] {
        assert_eq!(Plan::compile(&rs), Err(Failure::Invalid));
    }
}

#[test]
fn uncompressible_profile_stops_at_budget_without_an_alternative_manifest() {
    assert_eq!(
        Plan::compile(&rows(&[
            1_000_000_000_007,
            444_444_444_441,
            271_828_182_843,
            141_421_356_237
        ])),
        Err(Failure::Budget)
    );
}
